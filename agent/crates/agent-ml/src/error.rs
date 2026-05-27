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
}

pub type MlResult<T> = Result<T, MlError>;