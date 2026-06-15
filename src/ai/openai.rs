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

pub struct OpenAiProvider {
    pub api_key: String,
    pub base_url: String,
    pub default_model: String,
    pub use_responses_api: bool,
    pub client: Client,
}

impl OpenAiProvider {
    pub fn openai(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::compatible(api_key, "https://api.openai.com/v1", model, false)
    }

    pub fn compatible(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
        use_responses_api: bool,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: normalize_base_url(base_url.into(), "https://api.openai.com/v1"),
            default_model: model.into(),
            use_responses_api,
            client: Client::new(),
        }
    }

    pub fn openrouter(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            default_model: model.into(),
            use_responses_api: false,
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
            base_url: normalize_base_url(base_url.into(), "https://api.openai.com/v1"),
            default_model: model.into(),
            use_responses_api: false,
            client: Client::new(),
        }
    }

    fn should_use_max_completion_tokens(&self, model: &str) -> bool {
        let base = self.base_url.to_ascii_lowercase();
        let model = model.to_ascii_lowercase();
        base.contains("api.openai.com")
            || model.starts_with("gpt-5")
            || model.starts_with("o1")
            || model.starts_with("o3")
            || model.starts_with("o4")
    }

    fn is_reasoning_model(model: &str) -> bool {
        let model = model.to_ascii_lowercase();
        model.starts_with("gpt-5")
            || model.starts_with("o1")
            || model.starts_with("o3")
            || model.starts_with("o4")
    }

    fn apply_token_limit(&self, body: &mut Value, req: &ChatRequest) {
        if self.should_use_max_completion_tokens(&req.model) {
            body["max_completion_tokens"] = json!(req.max_tokens);
        } else {
            body["max_tokens"] = json!(req.max_tokens);
        }
    }

    fn force_max_completion_tokens(body: &mut Value, max_tokens: u32) {
        if let Some(obj) = body.as_object_mut() {
            obj.remove("max_tokens");
            obj.insert("max_completion_tokens".to_string(), json!(max_tokens));
        }
    }

    fn wants_max_completion_tokens(error_text: &str) -> bool {
        let text = error_text.to_ascii_lowercase();
        text.contains("max_tokens")
            && text.contains("max_completion_tokens")
            && (text.contains("unsupported") || text.contains("invalid"))
    }

    fn api_reasoning_effort(effort: &str) -> Option<&'static str> {
        match effort.trim().to_ascii_lowercase().as_str() {
            "" | "default" | "none" => None,
            "minimal" => Some("minimal"),
            "low" => Some("low"),
            "medium" => Some("medium"),
            "high" => Some("high"),
            "xhigh" | "x-high" | "x_high" => Some("xhigh"),
            _ => None,
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

        let is_reasoning_model = self.should_use_max_completion_tokens(&req.model);
        let mut body = json!({
            "model": req.model,
            "messages": messages,
            "stream": req.stream,
        });
        if !is_reasoning_model {
            body["temperature"] = json!(req.temperature);
        }
        self.apply_token_limit(&mut body, req);

        if is_reasoning_model {
            if let Some(effort) = req
                .reasoning_effort
                .as_deref()
                .and_then(Self::api_reasoning_effort)
            {
                body["reasoning_effort"] = json!(effort);
            }
        }

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

    fn build_responses_body(&self, req: &ChatRequest, stream: bool) -> Value {
        let mut input: Vec<Value> = Vec::new();
        for m in &req.messages {
            match m.role {
                ChatRole::System | ChatRole::User => {
                    let role_str = if m.role == ChatRole::System {
                        "system"
                    } else {
                        "user"
                    };
                    input.push(json!({
                        "role": role_str,
                        "content": m.content,
                    }));
                }
                ChatRole::Assistant => {
                    // If the assistant message has tool_calls, emit function_call items
                    if let Some(tcs) = &m.tool_calls {
                        for tc in tcs {
                            input.push(json!({
                                "type": "function_call",
                                "call_id": tc.id,
                                "name": tc.function.name,
                                "arguments": tc.function.arguments,
                            }));
                        }
                    }
                    // If the assistant also has text content, emit it as a message
                    if !m.content.is_empty() {
                        input.push(json!({
                            "role": "assistant",
                            "content": m.content,
                        }));
                    }
                }
                ChatRole::Tool => {
                    // Convert tool-role messages to function_call_output items
                    if let Some(call_id) = &m.tool_call_id {
                        input.push(json!({
                            "type": "function_call_output",
                            "call_id": call_id,
                            "output": m.content,
                        }));
                    }
                }
            }
        }

        let mut body = json!({
            "model": req.model,
            "input": input,
            "max_output_tokens": req.max_tokens,
            "stream": stream,
        });

        let effort = req
            .reasoning_effort
            .as_deref()
            .and_then(Self::api_reasoning_effort);
        if effort.is_some() || Self::is_reasoning_model(&req.model) {
            let mut reasoning = json!({ "summary": "auto" });
            if let Some(effort) = effort {
                reasoning["effort"] = json!(effort);
            }
            body["reasoning"] = reasoning;
        }

        // Attach tool definitions when present
        if let Some(tools) = &req.tools {
            body["tools"] = json!(tools
                .iter()
                .map(|t| json!({
                    "type": "function",
                    "name": t.function.name,
                    "description": t.function.description,
                    "parameters": t.function.parameters,
                }))
                .collect::<Vec<_>>());
        }

        body
    }

    fn should_use_responses_api(&self, _req: &ChatRequest) -> bool {
        self.use_responses_api
    }

    fn parse_responses_text(data: &Value) -> String {
        let mut reasoning = String::new();
        if let Some(items) = data["output"].as_array() {
            for item in items {
                if item["type"].as_str() != Some("reasoning") {
                    continue;
                }
                if let Some(summary) = item["summary"].as_array() {
                    for block in summary {
                        if let Some(text) = block["text"]
                            .as_str()
                            .or_else(|| block["summary_text"].as_str())
                        {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                if !reasoning.is_empty() {
                                    reasoning.push_str("\n\n");
                                }
                                if trimmed.len() <= 80 && !trimmed.contains('\n') {
                                    reasoning.push_str(&format!("**{}**", trimmed));
                                } else {
                                    reasoning.push_str(trimmed);
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut out = String::new();
        if !reasoning.trim().is_empty() {
            out.push_str("<thinking>\n");
            out.push_str(reasoning.trim());
            out.push_str("\n</thinking>\n");
        }

        if let Some(text) = data["output_text"].as_str() {
            out.push_str(text);
            return out;
        }

        if let Some(items) = data["output"].as_array() {
            for item in items {
                if let Some(content) = item["content"].as_array() {
                    for block in content {
                        if let Some(text) = block["text"].as_str() {
                            out.push_str(text);
                        }
                    }
                }
            }
        }
        out
    }

    /// Parse function_call items from a Responses API output array.
    fn parse_responses_tool_calls(data: &Value) -> Option<Vec<ToolCall>> {
        let items = data["output"].as_array()?;
        let tcs: Vec<ToolCall> = items
            .iter()
            .filter(|item| item["type"].as_str() == Some("function_call"))
            .filter_map(|item| {
                let call_id = item["call_id"].as_str()?.to_string();
                let name = item["name"].as_str()?.to_string();
                let args = item["arguments"].as_str().unwrap_or("{}").to_string();
                Some(ToolCall {
                    id: call_id,
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
        if self.should_use_responses_api(&request) {
            let body = self.build_responses_body(&request, false);
            let resp = self
                .client
                .post(format!("{}/responses", self.base_url))
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(anyhow!("Responses API error {}: {}", status, text));
            }

            let data: Value = resp.json().await?;
            let content = Self::parse_responses_text(&data);
            let model = data["model"].as_str().unwrap_or(&request.model).to_string();
            let prompt_tokens = data["usage"]["input_tokens"].as_u64().map(|n| n as u32);
            let completion_tokens = data["usage"]["output_tokens"].as_u64().map(|n| n as u32);
            let tool_calls = Self::parse_responses_tool_calls(&data);

            return Ok(ChatResponse {
                content,
                model,
                prompt_tokens,
                completion_tokens,
                tool_calls,
            });
        }

        let mut body = self.build_body(&request);
        body["stream"] = json!(false);

        let mut resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if body.get("max_tokens").is_some() && Self::wants_max_completion_tokens(&text) {
                Self::force_max_completion_tokens(&mut body, request.max_tokens);
                resp = self
                    .client
                    .post(format!("{}/chat/completions", self.base_url))
                    .bearer_auth(&self.api_key)
                    .json(&body)
                    .send()
                    .await?;
                if resp.status().is_success() {
                    let data: Value = resp.json().await?;
                    let mut content = String::new();
                    let reasoning = data["choices"][0]["message"]["reasoning_content"]
                        .as_str()
                        .or_else(|| data["choices"][0]["message"]["reasoning"].as_str());
                    if let Some(reasoning) = reasoning {
                        if !reasoning.trim().is_empty() {
                            content.push_str("<thinking>\n");
                            content.push_str(reasoning.trim());
                            content.push_str("\n</thinking>\n");
                        }
                    }
                    if let Some(text) = data["choices"][0]["message"]["content"].as_str() {
                        content.push_str(text);
                    }
                    let model = data["model"].as_str().unwrap_or(&request.model).to_string();
                    let prompt_tokens = data["usage"]["prompt_tokens"].as_u64().map(|n| n as u32);
                    let completion_tokens = data["usage"]["completion_tokens"]
                        .as_u64()
                        .map(|n| n as u32);
                    let tool_calls = Self::parse_tool_calls(&data);

                    return Ok(ChatResponse {
                        content,
                        model,
                        prompt_tokens,
                        completion_tokens,
                        tool_calls,
                    });
                }
                let retry_status = resp.status();
                let retry_text = resp.text().await.unwrap_or_default();
                return Err(anyhow!("API error {}: {}", retry_status, retry_text));
            }
            return Err(anyhow!("API error {}: {}", status, text));
        }

        let data: Value = resp.json().await?;
        let mut content = String::new();
        let reasoning = data["choices"][0]["message"]["reasoning_content"]
            .as_str()
            .or_else(|| data["choices"][0]["message"]["reasoning"].as_str());
        if let Some(reasoning) = reasoning {
            if !reasoning.trim().is_empty() {
                content.push_str("<thinking>\n");
                content.push_str(reasoning.trim());
                content.push_str("\n</thinking>\n");
            }
        }
        if let Some(text) = data["choices"][0]["message"]["content"].as_str() {
            content.push_str(text);
        }
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
        if self.should_use_responses_api(&request) {
            let body = self.build_responses_body(&request, true);
            let resp = self
                .client
                .post(format!("{}/responses", self.base_url))
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(anyhow!("Responses API error {}: {}", status, text));
            }

            let mut thinking_open = false;
            let stream = resp.bytes_stream().map(move |chunk| -> Result<String> {
                let bytes = chunk?;
                let text = std::str::from_utf8(&bytes).unwrap_or("");
                let mut content = String::new();
                for line in text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            if thinking_open {
                                content.push_str("</thinking>");
                                thinking_open = false;
                            }
                            break;
                        }
                        if let Ok(val) = serde_json::from_str::<Value>(data) {
                            let event_type = val["type"].as_str().unwrap_or("");
                            if event_type == "response.output_text.delta" {
                                if thinking_open {
                                    content.push_str("</thinking>\n");
                                    thinking_open = false;
                                }
                                if let Some(delta) = val["delta"].as_str() {
                                    content.push_str(delta);
                                }
                            } else if event_type.contains("reasoning")
                                && event_type.ends_with(".delta")
                            {
                                let delta = val["delta"]
                                    .as_str()
                                    .or_else(|| val["text"].as_str())
                                    .or_else(|| val["summary"].as_str());
                                if let Some(delta) = delta {
                                    if !thinking_open {
                                        content.push_str("<thinking>");
                                        thinking_open = true;
                                    }
                                    content.push_str(delta);
                                }
                            } else if thinking_open
                                && (event_type.contains("reasoning")
                                    && event_type.ends_with(".done")
                                    || event_type == "response.completed")
                            {
                                content.push_str("</thinking>");
                                thinking_open = false;
                            }
                        }
                    }
                }
                Ok(content)
            });

            return Ok(Box::pin(stream));
        }

        let mut body = self.build_body(&request);
        body["stream"] = json!(true);

        let mut resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if body.get("max_tokens").is_some() && Self::wants_max_completion_tokens(&text) {
                Self::force_max_completion_tokens(&mut body, request.max_tokens);
                resp = self
                    .client
                    .post(format!("{}/chat/completions", self.base_url))
                    .bearer_auth(&self.api_key)
                    .json(&body)
                    .send()
                    .await?;
                if resp.status().is_success() {
                    return Self::chat_completion_stream(resp).await;
                }
                let retry_status = resp.status();
                let retry_text = resp.text().await.unwrap_or_default();
                return Err(anyhow!("API error {}: {}", retry_status, retry_text));
            }
            return Err(anyhow!("API error {}: {}", status, text));
        }

        Self::chat_completion_stream(resp).await
    }
}

impl OpenAiProvider {
    async fn chat_completion_stream(resp: reqwest::Response) -> Result<ChatStream> {
        let mut thinking_open = false;
        let stream = resp.bytes_stream().map(move |chunk| -> Result<String> {
            let bytes = chunk?;
            let text = std::str::from_utf8(&bytes).unwrap_or("");
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        if thinking_open {
                            content.push_str("</thinking>");
                            thinking_open = false;
                        }
                        break;
                    }
                    if let Ok(val) = serde_json::from_str::<Value>(data) {
                        let reasoning_delta = val["choices"][0]["delta"]["reasoning_content"]
                            .as_str()
                            .or_else(|| val["choices"][0]["delta"]["reasoning"].as_str());
                        if let Some(delta) = reasoning_delta {
                            if !thinking_open {
                                content.push_str("<thinking>");
                                thinking_open = true;
                            }
                            content.push_str(delta);
                        } else if let Some(delta) = val["choices"][0]["delta"]["content"].as_str() {
                            if thinking_open {
                                content.push_str("</thinking>");
                                thinking_open = false;
                            }
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
