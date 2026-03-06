//! OpenAI-compatible provider. Works for OpenAI, Azure OpenAI, Ollama, vLLM, etc.
use crate::llm::LlmProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct OpenAiCompatProvider {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
    max_tokens: u32,
}

impl OpenAiCompatProvider {
    pub fn new(base_url: String, api_key: String, model: String, max_tokens: u32) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
            model,
            max_tokens,
        }
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Deserialize)]
struct AssistantMessage {
    content: String,
}

#[async_trait]
impl LlmProvider for OpenAiCompatProvider {
    async fn complete(&self, system: &str, user: &str) -> Result<String> {
        let request = ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system,
                },
                ChatMessage {
                    role: "user",
                    content: user,
                },
            ],
            max_tokens: self.max_tokens,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to reach OpenAI-compatible API")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("API error {}: {}", status, body));
        }

        let chat: ChatResponse = response
            .json()
            .await
            .context("Failed to parse API response")?;

        chat.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow::anyhow!("Empty choices in API response"))
    }
}
