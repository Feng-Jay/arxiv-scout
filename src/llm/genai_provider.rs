use crate::llm::LlmProvider;
use anyhow::Result;
use async_trait::async_trait;
use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use std::sync::Arc;

pub struct GenaiProvider {
    client: Client,
    model: String,
    max_tokens: u32,
}

impl GenaiProvider {
    pub fn new(
        adapter_kind: AdapterKind,
        endpoint: String,
        api_key: String,
        model: String,
        max_tokens: u32,
    ) -> Self {
        let endpoint: Arc<str> = endpoint.into();
        let api_key: Arc<str> = api_key.into();

        let target_resolver = ServiceTargetResolver::from_resolver_fn(
            move |svc: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
                let ep = Endpoint::from_owned(Arc::clone(&endpoint));
                let auth = AuthData::from_single(api_key.to_string());
                let mdl = ModelIden::new(adapter_kind, svc.model.model_name);
                Ok(ServiceTarget {
                    endpoint: ep,
                    auth,
                    model: mdl,
                })
            },
        );

        let client = Client::builder()
            .with_service_target_resolver(target_resolver)
            .build();

        Self {
            client,
            model,
            max_tokens,
        }
    }
}

#[async_trait]
impl LlmProvider for GenaiProvider {
    async fn complete(&self, system: &str, user: &str) -> Result<String> {
        let chat_req = ChatRequest::default()
            .with_system(system)
            .append_message(ChatMessage::user(user));

        let options = ChatOptions::default().with_max_tokens(self.max_tokens);

        let response = self
            .client
            .exec_chat(&self.model, chat_req, Some(&options))
            .await?;

        response
            .first_text()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Empty response from LLM"))
    }
}
