pub mod genai_provider;

use crate::config::ModelSlotConfig;
use anyhow::Result;
use async_trait::async_trait;
use genai::adapter::AdapterKind;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, system: &str, user: &str) -> Result<String>;
}

pub fn create_provider(slot: &ModelSlotConfig) -> Result<Box<dyn LlmProvider>> {
    let (adapter_kind, endpoint) = match slot.provider.as_str() {
        "openai" => {
            let base_url = slot
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1/".to_string());
            (AdapterKind::OpenAI, base_url)
        }
        "anthropic" => (
            AdapterKind::Anthropic,
            "https://api.anthropic.com/v1/".to_string(),
        ),
        "custom" => {
            let base_url = slot.base_url.clone().ok_or_else(|| {
                anyhow::anyhow!("'custom' provider requires base_url to be set")
            })?;
            (AdapterKind::OpenAI, base_url)
        }
        other => return Err(anyhow::anyhow!("Unknown LLM provider: '{}'", other)),
    };

    Ok(Box::new(genai_provider::GenaiProvider::new(
        adapter_kind,
        endpoint,
        slot.api_key.clone(),
        slot.model.clone(),
        slot.max_tokens,
    )))
}
