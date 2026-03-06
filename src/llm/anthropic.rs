use crate::llm::LlmProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
    max_tokens: u32,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String, max_tokens: u32) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            max_tokens,
        }
    }

    async fn post(&self, body: serde_json::Value) -> Result<String> {
        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .context("Failed to reach Anthropic API")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic API error {}: {}", status, body));
        }

        let resp: MessagesResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        resp.content
            .into_iter()
            .filter(|b| b.block_type == "text")
            .find_map(|b| b.text)
            .ok_or_else(|| anyhow::anyhow!("Empty text content in Anthropic response"))
    }
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Serialize)]
struct TextRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: [TextMessage<'a>; 1],
}

#[derive(Serialize)]
struct TextMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, system: &str, user: &str) -> Result<String> {
        let body = serde_json::to_value(TextRequest {
            model: &self.model,
            max_tokens: self.max_tokens,
            system,
            messages: [TextMessage {
                role: "user",
                content: user,
            }],
        })?;
        self.post(body).await
    }
}
