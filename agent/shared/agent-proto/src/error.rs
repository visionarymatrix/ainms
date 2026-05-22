use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("mTLS error: {0}")]
    Mtls(String),

    #[error("Classification error: {0}")]
    Classification(String),

    #[error("Store error: {0}")]
    Store(String),

    #[error("Watchdog error: {0}")]
    Watchdog(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}