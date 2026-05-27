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
        let messages: Vec<Value> = request.messages.iter().map(|m| {
            json!({
                "role": match m.role { ChatRole::System => "system", ChatRole::User => "user", ChatRole::Assistant => "assistant", ChatRole::Tool => "user" },
                "content": m.content
            })
        }).collect();

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
        let messages: Vec<Value> = request.messages.iter().map(|m| {
            json!({
                "role": match m.role { ChatRole::System => "system", ChatRole::User => "user", ChatRole::Assistant => "assistant", ChatRole::Tool => "user" },
                "content": m.content
            })
        }).collect();

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
