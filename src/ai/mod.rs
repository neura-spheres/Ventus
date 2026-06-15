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

fn provider_id(id: &str) -> &str {
    match id {
        "openai_compatible" | "openai-compatible" => "openai",
        "anthropic_compatible" | "anthropic-compatible" => "anthropic",
        "gemini_compatible" | "gemini-compatible" => "gemini",
        other => other,
    }
}

pub fn build_provider(settings: &AppSettings) -> Option<Arc<dyn AiProvider>> {
    let ai = &settings.ai;
    match provider_id(&ai.default_provider) {
        "openai" => {
            if let Ok(Some(key)) = crate::storage::keychain::get_api_key("openai") {
                Some(Arc::new(openai::OpenAiProvider::compatible(
                    key,
                    &ai.openai_base_url,
                    &ai.default_model,
                    ai.openai_use_responses_api,
                )))
            } else {
                None
            }
        }
        "anthropic" => {
            if let Ok(Some(key)) = crate::storage::keychain::get_api_key("anthropic") {
                Some(Arc::new(anthropic::AnthropicProvider::with_base_url(
                    key,
                    &ai.anthropic_base_url,
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
                Some(Arc::new(gemini::GeminiProvider::with_base_url(
                    key,
                    &ai.gemini_base_url,
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
