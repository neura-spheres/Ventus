use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};

use super::provider::*;

pub struct OllamaProvider {
    pub base_url: String,
    pub default_model: String,
    pub client: Client,
}

impl OllamaProvider {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            default_model: model.into(),
            client: Client::new(),
        }
    }

    fn message_json(message: &ChatMessage) -> Value {
        let mut content = message.content.clone();
        let mut images = Vec::new();
        for attachment in &message.attachments {
            if attachment.kind == AiAttachmentKind::Image {
                if let Some(data) = attachment.data_base64.as_ref() {
                    images.push(data.clone());
                }
                continue;
            }
            if let Some(text) = attachment.text_block() {
                if !content.is_empty() {
                    content.push_str("\n\n");
                }
                content.push_str(&text);
            }
        }
        let mut value = json!({
            "role": match message.role { ChatRole::System => "system", ChatRole::User => "user", ChatRole::Assistant => "assistant", ChatRole::Tool => "user" },
            "content": content
        });
        if !images.is_empty() {
            value["images"] = json!(images);
        }
        value
    }
}

#[async_trait::async_trait]
impl AiProvider for OllamaProvider {
    fn provider_id(&self) -> &'static str {
        "ollama"
    }
    fn provider_name(&self) -> &'static str {
        "Ollama"
    }
    fn supports_streaming(&self) -> bool {
        true
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let messages: Vec<Value> = request.messages.iter().map(Self::message_json).collect();

        let body = json!({
            "model": request.model,
            "messages": messages,
            "stream": false,
            "options": { "temperature": request.temperature, "num_predict": request.max_tokens }
        });

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Ollama error {}: {}", status, text));
        }

        let data: Value = resp.json().await?;
        let content = data["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Ok(ChatResponse {
            content,
            model: request.model.clone(),
            prompt_tokens: None,
            completion_tokens: None,
            tool_calls: None,
        })
    }

    async fn stream_chat(&self, request: ChatRequest) -> Result<ChatStream> {
        let messages: Vec<Value> = request.messages.iter().map(Self::message_json).collect();

        let body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
            "options": { "temperature": request.temperature }
        });

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Ollama error {}: {}", status, text));
        }

        let stream = resp.bytes_stream().map(|chunk| -> Result<String> {
            let bytes = chunk?;
            let text = std::str::from_utf8(&bytes).unwrap_or("");
            let mut content = String::new();
            for line in text.lines() {
                if let Ok(val) = serde_json::from_str::<Value>(line) {
                    if let Some(c) = val["message"]["content"].as_str() {
                        content.push_str(c);
                    }
                }
            }
            Ok(content)
        });

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_use_ollama_image_arrays() {
        let message = ChatMessage::user_with_attachments(
            "Describe",
            vec![AiAttachment {
                id: "image".into(),
                name: "photo.jpg".into(),
                mime_type: "image/jpeg".into(),
                kind: AiAttachmentKind::Image,
                size: 3,
                data_base64: Some("YWJj".into()),
                text: None,
            }],
        );
        let value = OllamaProvider::message_json(&message);
        assert_eq!(value["images"][0], "YWJj");
        assert_eq!(value["content"], "Describe");
    }
}
