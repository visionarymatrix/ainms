use async_trait::async_trait;

use crate::error::MlResult;
use crate::types::{AgentResponse, ChatGenerateOptions, ChatMessage, GenerateOptions};

/// Abstract interface for any LLM inference backend.
///
/// Implementations include `LlamaCppProvider` (behind `llama-cpp` feature)
/// and `MockProvider` (always available for testing).
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn provider_name(&self) -> &str;

    async fn load_model(&self, model_path: &str) -> MlResult<()>;

    fn is_loaded(&self) -> bool;

    async fn generate(&self, prompt: &str, options: &GenerateOptions) -> MlResult<String>;

    /// Chat-template-based generation with optional tool calling support.
    ///
    /// Takes a list of chat messages and options (which may include tool
    /// definitions). Returns either plain text or a request to call tools.
    ///
    /// The default implementation falls back to `generate()` with a
    /// concatenated prompt. Providers that support chat templates natively
    /// (e.g. `LlamaCppProvider`) should override this.
    async fn chat_generate(
        &self,
        messages: &[ChatMessage],
        options: &ChatGenerateOptions,
    ) -> MlResult<AgentResponse> {
        let prompt = messages
            .iter()
            .map(|m| format!("{}: {}", m.role.as_str(), m.content.text()))
            .collect::<Vec<_>>()
            .join("\n");

        let gen_opts = GenerateOptions {
            max_tokens: options.max_tokens,
            temperature: options.temperature,
            top_p: options.top_p,
            top_k: options.top_k,
            repeat_penalty: options.repeat_penalty,
            seed: options.seed,
            stop_strings: vec![],
        };

        let text = self.generate(&prompt, &gen_opts).await?;
        Ok(AgentResponse::Text(text))
    }

    /// Load a multimodal projection file for vision model support.
    ///
    /// Only meaningful for providers that support multimodal (e.g. LlamaCpp
    /// with the `mtmd` feature). Default is a no-op.
    async fn load_mmproj(&self, _mmproj_path: &str) -> MlResult<()> {
        Ok(())
    }
}