pub mod anthropic;
pub mod browser_tools;
pub mod chat;
pub mod gemini;
pub mod ollama;
pub mod openai;
pub mod prompts;
pub mod provider;
pub mod tools;

use crate::config::AppSettings;
use std::sync::Arc;

pub use provider::{AiProvider, ChatMessage, ChatRequest};

pub fn build_provider(settings: &AppSettings) -> Option<Arc<dyn AiProvider>> {
    let ai = &settings.ai;
    match ai.default_provider.as_str() {
        "openai" => {
            if let Ok(Some(key)) = crate::storage::keychain::get_api_key("openai") {
                Some(Arc::new(openai::OpenAiProvider::openai(
                    key,
                    &ai.default_model,
                )))
            } else {
                None
            }
        }
        "anthropic" => {
            if let Ok(Some(key)) = crate::storage::keychain::get_api_key("anthropic") {
                Some(Arc::new(anthropic::AnthropicProvider::new(
                    key,
                    &ai.default_model,
                )))
            } else {
                None
            }
        }
        "openrouter" => {
            if let Ok(Some(key)) = crate::storage::keychain::get_api_key("openrouter") {
                Some(Arc::new(openai::OpenAiProvider::openrouter(
                    key,
                    &ai.default_model,
                )))
            } else {
                None
            }
        }
        "gemini" => {
            if let Ok(Some(key)) = crate::storage::keychain::get_api_key("gemini") {
                Some(Arc::new(gemini::GeminiProvider::new(
                    key,
                    &ai.default_model,
                )))
            } else {
                None
            }
        }
        "ollama" => {
            let base_url = "http://localhost:11434";
            Some(Arc::new(ollama::OllamaProvider::new(
                base_url,
                &ai.default_model,
            )))
        }
        _ => None,
    }
}
