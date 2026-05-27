use async_trait::async_trait;
use std::sync::Mutex;

use crate::error::MlResult;
use crate::provider::LlmProvider;
use crate::types::GenerateOptions;
use crate::ClassificationResult;

/// A mock LLM provider for testing.
///
/// Returns pre-configured responses, useful for unit-testing code that
/// depends on `dyn LlmProvider` without requiring an actual model file.
pub struct MockProvider {
    loaded: std::sync::atomic::AtomicBool,
    responses: Mutex<Vec<String>>,
    classifications: Mutex<Vec<ClassificationResult>>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            loaded: std::sync::atomic::AtomicBool::new(false),
            responses: Mutex::new(Vec::new()),
            classifications: Mutex::new(Vec::new()),
        }
    }

    /// Pre-configure a text generation response. Responses are consumed
    /// in FIFO order; if the queue is empty, `"mock response"` is returned.
    pub fn with_response(self, response: &str) -> Self {
        self.responses.lock().unwrap().push(response.to_string());
        self
    }

    /// Pre-configure a classification result. Results are consumed in
    /// FIFO order; if the queue is empty, a default neutral result is
    /// returned.
    pub fn with_classification(self, result: ClassificationResult) -> Self {
        self.classifications.lock().unwrap().push(result);
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
}