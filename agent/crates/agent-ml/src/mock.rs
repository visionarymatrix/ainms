use async_trait::async_trait;
use std::sync::Mutex;

use crate::error::MlResult;
use crate::provider::LlmProvider;
use crate::types::{AgentResponse, ChatGenerateOptions, ChatMessage, GenerateOptions};
use crate::ClassificationResult;

pub struct MockProvider {
    loaded: std::sync::atomic::AtomicBool,
    responses: Mutex<Vec<String>>,
    classifications: Mutex<Vec<ClassificationResult>>,
    chat_responses: Mutex<Vec<AgentResponse>>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            loaded: std::sync::atomic::AtomicBool::new(false),
            responses: Mutex::new(Vec::new()),
            classifications: Mutex::new(Vec::new()),
            chat_responses: Mutex::new(Vec::new()),
        }
    }

    pub fn with_response(self, response: &str) -> Self {
        self.responses.lock().unwrap().push(response.to_string());
        self
    }

    pub fn with_classification(self, result: ClassificationResult) -> Self {
        self.classifications.lock().unwrap().push(result);
        self
    }

    pub fn with_chat_response(self, response: AgentResponse) -> Self {
        self.chat_responses.lock().unwrap().push(response);
        self
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    fn provider_name(&self) -> &str {
        "mock"
    }

    async fn load_model(&self, _model_path: &str) -> MlResult<()> {
        self.loaded
            .store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    fn is_loaded(&self) -> bool {
        self.loaded.load(std::sync::atomic::Ordering::Relaxed)
    }

    async fn generate(&self, _prompt: &str, _options: &GenerateOptions) -> MlResult<String> {
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            Ok("mock response".to_string())
        } else {
            Ok(responses.remove(0))
        }
    }

    async fn chat_generate(
        &self,
        _messages: &[ChatMessage],
        _options: &ChatGenerateOptions,
    ) -> MlResult<AgentResponse> {
        let mut chat_responses = self.chat_responses.lock().unwrap();
        if chat_responses.is_empty() {
            Ok(AgentResponse::Text("mock chat response".to_string()))
        } else {
            Ok(chat_responses.remove(0))
        }
    }
}