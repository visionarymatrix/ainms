use async_trait::async_trait;

use crate::error::MlResult;
use crate::types::GenerateOptions;

/// Abstract interface for any LLM inference backend.
///
/// Implementations include `LlamaCppProvider` (behind `llama-cpp` feature)
/// and `MockProvider` (always available for testing).
///
/// # Swapping backends
///
/// Because all provider logic sits behind this trait, switching from
/// `llama-cpp-2` to a different llama.cpp binding (or a different inference
/// engine entirely) only requires writing a new `impl LlmProvider` — no
/// call-site changes needed.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Human-readable name of this backend (e.g. "llama-cpp", "mock").
    fn provider_name(&self) -> &str;

    /// Load a GGUF model from disk.
    ///
    /// Must be called before `generate()`. Calling it again replaces the
    /// previously loaded model.
    async fn load_model(&self, model_path: &str) -> MlResult<()>;

    /// Check whether a model has been loaded.
    fn is_loaded(&self) -> bool;

    /// Generate text completion from a prompt.
    ///
    /// The prompt is tokenised, fed through the model, and sampled according
    /// to `options`. Returns the generated text **without** the original
    /// prompt.
    async fn generate(&self, prompt: &str, options: &GenerateOptions) -> MlResult<String>;
}