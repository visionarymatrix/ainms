use thiserror::Error;

/// Errors that can occur during ML inference operations.
#[derive(Debug, Error)]
pub enum MlError {
    #[error("LLM provider error: {0}")]
    Provider(String),

    #[error("Model not loaded — call load_model() first")]
    NotLoaded,

    #[error("Tokenization failed: {0}")]
    Tokenization(String),

    #[error("Generation failed: {0}")]
    Generation(String),

    #[error("Invalid configuration: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // ── Multimodal errors ────────────────────────────────────────────────

    #[error("Multimodal error: {0}")]
    Multimodal(String),

    #[error("Image processing failed: {0}")]
    ImageProcessing(String),

    #[error("Base64 decode failed: {0}")]
    Base64Decode(String),

    // ── Tool calling errors ─────────────────────────────────────────────

    #[error("Tool calling error: {0}")]
    ToolCalling(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool execution failed: {0}")]
    ToolExecution(String),

    #[error("Failed to parse tool call from model output: {0}")]
    ToolCallParse(String),

    // ── Chat / Agent errors ─────────────────────────────────────────────

    #[error("Chat template error: {0}")]
    ChatTemplate(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Maximum agent iterations reached")]
    MaxIterationsReached,

    #[error("Tool search error: {0}")]
    ToolSearch(String),
}

pub type MlResult<T> = Result<T, MlError>;