use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};

use super::provider::*;

pub struct OpenAiProvider {
    pub api_key: String,
    pub base_url: String,
    pub default_model: String,
    pub client: Client,
}

impl OpenAiProvider {
    pub fn openai(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            default_model: model.into(),
            client: Client::new(),
        }
    }

    pub fn openrouter(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            default_model: model.into(),
            client: Client::new(),
        }
    }

    pub fn custom(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            default_model: model.into(),
            client: Client::new(),
        }
    }

    fn build_body(&self, req: &ChatRequest) -> Value {
        let messages: Vec<Value> = req
            .messages
            .iter()
            .map(|m| {
                let role_str = match m.role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "tool",
                };
                let mut msg = json!({ "role": role_str });

                // Content: use null for assistant messages that only have tool_calls
                if m.role == ChatRole::Assistant && m.content.is_empty() && m.tool_calls.is_some() {
                    msg["content"] = Value::Null;
                } else {
                    msg["content"] = json!(m.content);
                }

                // Tool call request (assistant → model)
                if let Some(tcs) = &m.tool_calls {
                    msg["tool_calls"] = json!(tcs
                        .iter()
                        .map(|tc| json!({
                            "id": tc.id,
                            "type": tc.call_type,
                            "function": {
                                "name": tc.function.name,
                                "arguments": tc.function.arguments,
                            }
                        }))
                        .collect::<Vec<_>>());
                }

                // Tool result (tool → model)
                if let Some(tcid) = &m.tool_call_id {
                    msg["tool_call_id"] = json!(tcid);
                }

                msg
            })
            .collect();

        let mut body = json!({
            "model": req.model,
            "messages": messages,
            "temperature": req.temperature,
            "max_tokens": req.max_tokens,
            "stream": req.stream,
        });

        // Attach tool definitions when present
        if let Some(tools) = &req.tools {
            body["tools"] = json!(tools
                .iter()
                .map(|t| json!({
                    "type": "function",
                    "function": {
                        "name": t.function.name,
                        "description": t.function.description,
                        "parameters": t.function.parameters,
                    }
                }))
                .collect::<Vec<_>>());
        }

        body
    }

    fn parse_tool_calls(data: &Value) -> Option<Vec<ToolCall>> {
        let arr = data["choices"][0]["message"]["tool_calls"].as_array()?;
        let tcs: Vec<ToolCall> = arr
            .iter()
            .filter_map(|tc| {
                let id = tc["id"].as_str()?.to_string();
                let name = tc["function"]["name"].as_str()?.to_string();
                let args = tc["function"]["arguments"]
                    .as_str()
                    .unwrap_or("{}")
                    .to_string();
                Some(ToolCall {
                    id,
                    call_type: "function".to_string(),
                    function: ToolCallFunction {
                        name,
                        arguments: args,
                    },
                })
            })
            .collect();
        if tcs.is_empty() {
            None
        } else {
            Some(tcs)
        }
    }
}

#[async_trait::async_trait]
impl AiProvider for OpenAiProvider {
    fn provider_id(&self) -> &'static str {
        "openai"
    }
    fn provider_name(&self) -> &'static str {
        "OpenAI"
    }
    fn supports_streaming(&self) -> bool {
        true
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let mut body = self.build_body(&request);
        body["stream"] = json!(false);

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("API error {}: {}", status, text));
        }

        let data: Value = resp.json().await?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let model = data["model"].as_str().unwrap_or(&request.model).to_string();
        let prompt_tokens = data["usage"]["prompt_tokens"].as_u64().map(|n| n as u32);
        let completion_tokens = data["usage"]["completion_tokens"]
            .as_u64()
            .map(|n| n as u32);
        let tool_calls = Self::parse_tool_calls(&data);

        Ok(ChatResponse {
            content,
            model,
            prompt_tokens,
            completion_tokens,
            tool_calls,
        })
    }

    async fn stream_chat(&self, request: ChatRequest) -> Result<ChatStream> {
        let mut body = self.build_body(&request);
        body["stream"] = json!(true);

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("API error {}: {}", status, text));
        }

        let stream = resp.bytes_stream().map(move |chunk| -> Result<String> {
            let bytes = chunk?;
            let text = std::str::from_utf8(&bytes).unwrap_or("");
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        break;
                    }
                    if let Ok(val) = serde_json::from_str::<Value>(data) {
                        if let Some(delta) = val["choices"][0]["delta"]["content"].as_str() {
                            content.push_str(delta);
                        }
                    }
                }
            }
            Ok(content)
        });

        Ok(Box::pin(stream))
    }
}
