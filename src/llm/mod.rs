pub mod anthropic;
pub mod openai_compat;

use crate::config::ModelSlotConfig;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, system: &str, user: &str) -> Result<String>;
}

pub fn create_provider(slot: &ModelSlotConfig) -> Result<Box<dyn LlmProvider>> {
    match slot.provider.as_str() {
        "openai" => {
            let base_url = slot
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            Ok(Box::new(openai_compat::OpenAiCompatProvider::new(
                base_url,
                slot.api_key.clone(),
                slot.model.clone(),
                slot.max_tokens,
            )))
        }
        "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new(
            slot.api_key.clone(),
            slot.model.clone(),
            slot.max_tokens,
        ))),
        "custom" => {
            let base_url = slot.base_url.clone().ok_or_else(|| {
                anyhow::anyhow!("'custom' provider requires base_url to be set")
            })?;
            Ok(Box::new(openai_compat::OpenAiCompatProvider::new(
                base_url,
                slot.api_key.clone(),
                slot.model.clone(),
                slot.max_tokens,
            )))
        }
        other => Err(anyhow::anyhow!("Unknown LLM provider: '{}'", other)),
    }
}
