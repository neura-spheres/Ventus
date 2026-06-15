use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};

use super::provider::*;

fn normalize_base_url(value: String, default_url: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        default_url.to_string()
    } else {
        trimmed
    }
}

pub struct AnthropicProvider {
    pub api_key: String,
    pub base_url: String,
    pub default_model: String,
    pub client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::with_base_url(api_key, "https://api.anthropic.com/v1", model)
    }

    pub fn with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: normalize_base_url(base_url.into(), "https://api.anthropic.com/v1"),
            default_model: model.into(),
            client: Client::new(),
        }
    }

    fn build_body(&self, req: &ChatRequest, stream: bool) -> Value {
        let mut system = String::new();
        let mut messages: Vec<Value> = vec![];

        let mut i = 0;
        while i < req.messages.len() {
            let m = &req.messages[i];
            match m.role {
                ChatRole::System => {
                    system = m.content.clone();
                    i += 1;
                }
                ChatRole::User => {
                    messages.push(json!({"role": "user", "content": m.content}));
                    i += 1;
                }
                ChatRole::Assistant => {
                    if let Some(tcs) = &m.tool_calls {
                        // Anthropic: assistant message with tool_use content blocks
                        let mut content: Vec<Value> = vec![];
                        if !m.content.is_empty() {
                            content.push(json!({"type": "text", "text": m.content}));
                        }
                        for tc in tcs {
                            let input: Value =
                                serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));
                            content.push(json!({
                                "type": "tool_use",
                                "id": tc.id,
                                "name": tc.function.name,
                                "input": input,
                            }));
                        }
                        messages.push(json!({"role": "assistant", "content": content}));
                    } else {
                        messages.push(json!({"role": "assistant", "content": m.content}));
                    }
                    i += 1;
                }
                ChatRole::Tool => {
                    // Anthropic: group consecutive tool-result messages into one user message
                    let mut tool_results: Vec<Value> = vec![];
                    while i < req.messages.len() && req.messages[i].role == ChatRole::Tool {
                        let tm = &req.messages[i];
                        tool_results.push(json!({
                            "type": "tool_result",
                            "tool_use_id": tm.tool_call_id.as_deref().unwrap_or(""),
                            "content": tm.content,
                        }));
                        i += 1;
                    }
                    messages.push(json!({"role": "user", "content": tool_results}));
                }
            }
        }

        let mut body = json!({
            "model": req.model,
            "messages": messages,
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
            "stream": stream,
        });

        if !system.is_empty() {
            body["system"] = json!(system);
        }

        // Anthropic tool format: name + description + input_schema (same JSON Schema as our parameters)
        if let Some(tools) = &req.tools {
            body["tools"] = json!(tools
                .iter()
                .map(|t| json!({
                    "name": t.function.name,
                    "description": t.function.description,
                    "input_schema": t.function.parameters,
                }))
                .collect::<Vec<_>>());
        }

        body
    }

    fn parse_content_blocks(data: &Value) -> (String, Option<Vec<ToolCall>>) {
        let Some(arr) = data["content"].as_array() else {
            let text = data["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string();
            return (text, None);
        };

        let mut text = String::new();
        let mut tool_calls: Vec<ToolCall> = vec![];

        for block in arr {
            match block["type"].as_str() {
                Some("text") => {
                    if let Some(t) = block["text"].as_str() {
                        text.push_str(t);
                    }
                }
                Some("tool_use") => {
                    if let (Some(id), Some(name)) = (block["id"].as_str(), block["name"].as_str()) {
                        let args = serde_json::to_string(&block["input"])
                            .unwrap_or_else(|_| "{}".to_string());
                        tool_calls.push(ToolCall {
                            id: id.to_string(),
                            call_type: "function".to_string(),
                            function: ToolCallFunction {
                                name: name.to_string(),
                                arguments: args,
                            },
                        });
                    }
                }
                _ => {}
            }
        }

        let tcs = if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        };
        (text, tcs)
    }
}

#[async_trait::async_trait]
impl AiProvider for AnthropicProvider {
    fn provider_id(&self) -> &'static str {
        "anthropic"
    }
    fn provider_name(&self) -> &'static str {
        "Anthropic"
    }
    fn supports_streaming(&self) -> bool {
        true
    }

    /// Uses Anthropic's built-in `web_search_20250305` server-side tool so the
    /// model answers with live search results without us executing any search.
    async fn spotlight_chat(&self, req: ChatRequest) -> Result<Option<String>> {
        let mut body = self.build_body(&req, false);
        // Attach Anthropic's server-executed web-search tool.
        body["tools"] = json!([{
            "type": "web_search_20250305",
            "name": "web_search",
            "max_uses": 3
        }]);

        let resp = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "web-search-2025-03-05")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Anthropic web-search error {}: {}", status, text));
        }

        let data: Value = resp.json().await?;

        // Response may contain web_search_tool_result blocks (search sources) and
        // text blocks (the grounded answer).  Collect only the text blocks.
        let mut answer = String::new();
        if let Some(arr) = data["content"].as_array() {
            for block in arr {
                if block["type"].as_str() == Some("text") {
                    if let Some(t) = block["text"].as_str() {
                        answer.push_str(t);
                    }
                }
            }
        }

        if answer.is_empty() {
            return Ok(None);
        }
        Ok(Some(answer))
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let body = self.build_body(&request, false);

        let resp = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Anthropic API error {}: {}", status, text));
        }

        let data: Value = resp.json().await?;
        let (content, tool_calls) = Self::parse_content_blocks(&data);
        let model = data["model"].as_str().unwrap_or(&request.model).to_string();
        let prompt_tokens = data["usage"]["input_tokens"].as_u64().map(|n| n as u32);
        let completion_tokens = data["usage"]["output_tokens"].as_u64().map(|n| n as u32);

        Ok(ChatResponse {
            content,
            model,
            prompt_tokens,
            completion_tokens,
            tool_calls,
        })
    }

    async fn stream_chat(&self, request: ChatRequest) -> Result<ChatStream> {
        let body = self.build_body(&request, true);

        let resp = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Anthropic API error {}: {}", status, text));
        }

        let mut thinking_open = false;
        let stream = resp.bytes_stream().map(move |chunk| -> Result<String> {
            let bytes = chunk?;
            let text = std::str::from_utf8(&bytes).unwrap_or("");
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(val) = serde_json::from_str::<Value>(data) {
                        if val["type"] == "content_block_delta" {
                            if let Some(delta) = val["delta"]["thinking"].as_str() {
                                if !thinking_open {
                                    content.push_str("<thinking>");
                                    thinking_open = true;
                                }
                                content.push_str(delta);
                            } else if let Some(delta) = val["delta"]["text"].as_str() {
                                if thinking_open {
                                    content.push_str("</thinking>\n");
                                    thinking_open = false;
                                }
                                content.push_str(delta);
                            }
                        } else if thinking_open
                            && (val["type"] == "content_block_stop"
                                || val["type"] == "message_stop")
                        {
                            content.push_str("</thinking>");
                            thinking_open = false;
                        }
                    }
                }
            }
            Ok(content)
        });

        Ok(Box::pin(stream))
    }
}
