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

pub struct GeminiProvider {
    pub api_key: String,
    pub base_url: String,
    pub client: Client,
}

impl GeminiProvider {
    pub fn new(api_key: impl Into<String>, _model: impl Into<String>) -> Self {
        Self::with_base_url(
            api_key,
            "https://generativelanguage.googleapis.com/v1beta",
            _model,
        )
    }

    pub fn with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        _model: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: normalize_base_url(
                base_url.into(),
                "https://generativelanguage.googleapis.com/v1beta",
            ),
            client: Client::new(),
        }
    }

    fn model_path(&self, model: &str) -> String {
        if model.starts_with("models/") {
            return model.to_string();
        }
        format!("models/{model}")
    }

    fn url(&self, model: &str, action: &str) -> String {
        format!("{}/{}:{}", self.base_url, self.model_path(model), action)
    }

    fn build_body(&self, req: &ChatRequest) -> Value {
        let mut system = String::new();
        let mut contents = vec![];

        for m in &req.messages {
            match m.role {
                ChatRole::System => {
                    if !system.is_empty() {
                        system.push('\n');
                    }
                    system.push_str(&m.content);
                }
                ChatRole::User => contents.push(json!({
                    "role": "user",
                    "parts": Self::user_parts(m)
                })),
                ChatRole::Assistant => contents.push(json!({
                    "role": "model",
                    "parts": [{"text": m.content}]
                })),
                // Gemini doesn't support native tool calls — fold results into user turn
                ChatRole::Tool => contents.push(json!({
                    "role": "user",
                    "parts": [{"text": format!("[Tool result]: {}", m.content)}]
                })),
            }
        }

        let mut body = json!({
            "contents": contents,
            "generationConfig": {
                "temperature": req.temperature,
                "maxOutputTokens": req.max_tokens
            }
        });

        if let Some(thinking_config) = Self::thinking_config(req) {
            body["generationConfig"]["thinkingConfig"] = thinking_config;
        }

        if !system.is_empty() {
            body["system_instruction"] = json!({
                "parts": [{"text": system}]
            });
        }

        body
    }

    fn user_parts(message: &ChatMessage) -> Vec<Value> {
        let mut parts = Vec::new();
        for attachment in &message.attachments {
            match attachment.kind {
                AiAttachmentKind::Image | AiAttachmentKind::Pdf => {
                    let Some(data) = attachment.data_base64.as_ref() else {
                        continue;
                    };
                    parts.push(json!({"text": format!("Attached file: {}", attachment.name)}));
                    parts.push(json!({
                        "inline_data": {
                            "mime_type": attachment.mime_type,
                            "data": data,
                        }
                    }));
                }
                AiAttachmentKind::Text => {
                    if let Some(text) = attachment.text_block() {
                        parts.push(json!({"text": text}));
                    }
                }
            }
        }
        if !message.content.trim().is_empty() {
            parts.push(json!({"text": message.content}));
        }
        parts
    }

    fn thinking_config(req: &ChatRequest) -> Option<Value> {
        if !Self::supports_thinking_config(&req.model) {
            return None;
        }

        let mut config = json!({ "includeThoughts": true });
        let effort = req.reasoning_effort.as_deref().unwrap_or("default");

        if Self::supports_thinking_level(&req.model) {
            if let Some(level) = Self::api_thinking_level(effort) {
                config["thinkingLevel"] = json!(level);
            }
        } else if Self::supports_thinking_budget(&req.model) {
            if let Some(budget) = Self::api_thinking_budget(effort) {
                config["thinkingBudget"] = json!(budget);
            }
        }

        Some(config)
    }

    fn supports_thinking_config(model: &str) -> bool {
        let model = model.to_ascii_lowercase();
        model.contains("gemini-3") || model.contains("gemini-2.5")
    }

    fn supports_thinking_level(model: &str) -> bool {
        model.to_ascii_lowercase().contains("gemini-3")
    }

    fn supports_thinking_budget(model: &str) -> bool {
        model.to_ascii_lowercase().contains("gemini-2.5")
    }

    fn api_thinking_level(effort: &str) -> Option<&'static str> {
        match effort.trim().to_ascii_lowercase().as_str() {
            "" | "default" => None,
            "none" | "minimal" => Some("MINIMAL"),
            "low" => Some("LOW"),
            "medium" => Some("MEDIUM"),
            "high" | "xhigh" | "x-high" | "x_high" => Some("HIGH"),
            _ => None,
        }
    }

    fn api_thinking_budget(effort: &str) -> Option<i32> {
        match effort.trim().to_ascii_lowercase().as_str() {
            "" | "default" => None,
            "none" => Some(0),
            "minimal" => Some(128),
            "low" => Some(512),
            "medium" => Some(1024),
            "high" | "xhigh" | "x-high" | "x_high" => Some(2048),
            _ => None,
        }
    }

    fn search_req(&self, req: &ChatRequest) -> ChatRequest {
        let mut req = req.clone();
        let query = req
            .messages
            .iter()
            .rev()
            .find(|m| m.role == ChatRole::User)
            .map(|m| m.content.clone())
            .unwrap_or_default();
        let search_rule = "You must use Google Search before answering every prompt. Do not answer from memory. If the prompt seems simple, still use Google Search first.";

        if let Some(system) = req.messages.iter_mut().find(|m| m.role == ChatRole::System) {
            system.content = format!("{search_rule}\n{}", system.content);
        } else {
            req.messages.insert(0, ChatMessage::system(search_rule));
        }

        if let Some(user) = req
            .messages
            .iter_mut()
            .rev()
            .find(|m| m.role == ChatRole::User)
        {
            user.content = format!("Use Google Search now, then answer this prompt:\n\n{query}");
        }

        req
    }

    fn search_tools(model: &str) -> Value {
        if model.contains("gemini-1.5") {
            return json!([{"google_search_retrieval": {}}]);
        }
        json!([{"google_search": {}}])
    }

    fn has_grounding(data: &Value) -> bool {
        data["candidates"][0]["groundingMetadata"]["webSearchQueries"]
            .as_array()
            .map(|queries| !queries.is_empty())
            .unwrap_or(false)
            || data["candidates"][0]["groundingMetadata"]["groundingChunks"]
                .as_array()
                .map(|chunks| !chunks.is_empty())
                .unwrap_or(false)
    }

    fn read_content(data: &Value) -> String {
        let Some(parts) = data["candidates"][0]["content"]["parts"].as_array() else {
            return String::new();
        };

        let mut thinking = String::new();
        let mut answer = String::new();
        for part in parts {
            let Some(text) = part["text"].as_str() else {
                continue;
            };
            if part["thought"].as_bool().unwrap_or(false) {
                thinking.push_str(text);
            } else {
                answer.push_str(text);
            }
        }

        if thinking.trim().is_empty() {
            return answer;
        }

        format!("<thinking>\n{}\n</thinking>\n{}", thinking.trim(), answer)
    }

    fn read_answer_text(data: &Value) -> String {
        data["candidates"][0]["content"]["parts"]
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter(|p| !p["thought"].as_bool().unwrap_or(false))
                    .filter_map(|p| p["text"].as_str())
                    .collect::<String>()
            })
            .unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl AiProvider for GeminiProvider {
    fn provider_id(&self) -> &'static str {
        "gemini"
    }

    fn provider_name(&self) -> &'static str {
        "Gemini"
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_native_pdf(&self) -> bool {
        true
    }

    async fn spotlight_chat(&self, req: ChatRequest) -> Result<Option<String>> {
        let req = self.search_req(&req);
        let mut body = self.build_body(&req);
        body["tools"] = Self::search_tools(&req.model);

        let resp = self
            .client
            .post(self.url(&req.model, "generateContent"))
            .query(&[("key", &self.api_key)])
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini search error {}: {}", status, text));
        }

        let data: Value = resp.json().await?;
        let text = Self::read_answer_text(&data);
        if text.is_empty() || !Self::has_grounding(&data) {
            return Ok(None);
        }
        Ok(Some(text))
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let body = self.build_body(&request);
        let resp = self
            .client
            .post(self.url(&request.model, "generateContent"))
            .query(&[("key", &self.api_key)])
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini error {}: {}", status, text));
        }

        let data: Value = resp.json().await?;
        let content = Self::read_content(&data);
        let prompt_tokens = data["usageMetadata"]["promptTokenCount"]
            .as_u64()
            .map(|n| n as u32);
        let completion_tokens = data["usageMetadata"]["candidatesTokenCount"]
            .as_u64()
            .map(|n| n as u32);

        Ok(ChatResponse {
            content,
            model: request.model.clone(),
            prompt_tokens,
            completion_tokens,
            tool_calls: None,
        })
    }

    async fn stream_chat(&self, request: ChatRequest) -> Result<ChatStream> {
        let body = self.build_body(&request);
        let resp = self
            .client
            .post(self.url(&request.model, "streamGenerateContent"))
            .query(&[("alt", "sse"), ("key", &self.api_key)])
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini error {}: {}", status, text));
        }

        let mut byte_stream = resp.bytes_stream();
        let stream = async_stream::try_stream! {
            let mut buffer = String::new();
            while let Some(chunk) = byte_stream.next().await {
                let bytes = chunk?;
                let text = std::str::from_utf8(&bytes).unwrap_or("");
                buffer.push_str(text);

                while let Some(line_end) = buffer.find('\n') {
                    let mut line = buffer[..line_end].trim_end_matches('\r').to_string();
                    buffer.drain(..=line_end);
                    if line.is_empty() {
                        continue;
                    }
                    let Some(data) = line.strip_prefix("data: ") else {
                        continue;
                    };
                    if data == "[DONE]" {
                        return;
                    }
                    let val = serde_json::from_str::<Value>(data)?;
                    let content = Self::read_content(&val);
                    if !content.is_empty() {
                        yield content;
                    }
                    line.clear();
                }
            }

            let tail = buffer.trim();
            if let Some(data) = tail.strip_prefix("data: ") {
                if !data.is_empty() && data != "[DONE]" {
                    let val = serde_json::from_str::<Value>(data)?;
                    let content = Self::read_content(&val);
                    if !content.is_empty() {
                        yield content;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_parts_include_inline_images_and_text_files() {
        let message = ChatMessage::user_with_attachments(
            "Compare these",
            vec![
                AiAttachment {
                    id: "image".into(),
                    name: "photo.png".into(),
                    mime_type: "image/png".into(),
                    kind: AiAttachmentKind::Image,
                    size: 3,
                    data_base64: Some("YWJj".into()),
                    text: None,
                },
                AiAttachment {
                    id: "text".into(),
                    name: "notes.txt".into(),
                    mime_type: "text/plain".into(),
                    kind: AiAttachmentKind::Text,
                    size: 5,
                    data_base64: None,
                    text: Some("hello".into()),
                },
            ],
        );
        let parts = GeminiProvider::user_parts(&message);
        assert!(parts
            .iter()
            .any(|part| part["inline_data"]["mime_type"] == "image/png"));
        assert!(parts.iter().any(|part| part["text"] == "Compare these"));
        assert!(parts.iter().any(|part| part["text"]
            .as_str()
            .unwrap_or_default()
            .contains("notes.txt")));
    }
}
