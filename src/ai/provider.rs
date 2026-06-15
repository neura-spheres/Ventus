use anyhow::Result;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

// ── Tool schemas (sent to AI to describe what it can call) ────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

impl Tool {
    pub fn function(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: name.into(),
                description: description.into(),
                parameters,
            },
        }
    }
}

// ── Tool calls (returned by the AI requesting a function call) ─────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    /// Raw JSON string with arguments
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

// ── Message ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    /// Set on assistant messages that contain tool call requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Set on Tool-role messages (the result of a tool call)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    /// Assistant message that requests tool calls (content may be empty)
    pub fn assistant_with_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: String::new(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }
    /// Tool result message to be fed back to the AI
    pub fn tool_result(call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(call_id.into()),
        }
    }
}

// ── Request / Response ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub stream: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    /// Optional list of tools the model may call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    /// Set when the model wants to call one or more tools instead of (or before) responding
    pub tool_calls: Option<Vec<ToolCall>>,
}

pub type ChatStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

// ── Provider trait ─────────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn provider_name(&self) -> &'static str;
    fn supports_streaming(&self) -> bool;

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    async fn stream_chat(&self, request: ChatRequest) -> Result<ChatStream>;

    /// Try to answer using this provider's native web-search capability.
    ///
    /// Returns `Ok(Some(text))` if native search produced a grounded answer,
    /// `Ok(None)` if this provider doesn't support native search,
    /// `Err(…)` on an API error (caller should fall back to Wikipedia).
    async fn spotlight_chat(&self, _req: ChatRequest) -> Result<Option<String>> {
        Ok(None)
    }

    async fn test_connection(&self) -> Result<String> {
        let req = ChatRequest {
            messages: vec![ChatMessage::user("Say 'OK' in one word.")],
            model: "gpt-4o-mini".to_string(),
            temperature: 0.0,
            max_tokens: 10,
            stream: false,
            reasoning_effort: None,
            tools: None,
        };
        let resp = self.chat(req).await?;
        Ok(format!(
            "Connected! Model responded: {}",
            resp.content.trim()
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProviderType {
    OpenAi,
    Anthropic,
    Gemini,
    OpenRouter,
    Ollama,
    Custom,
}
