use std::path::PathBuf;

/// Which LLM backend to instantiate.
#[derive(Debug, Clone, Default)]
pub enum ProviderType {
    #[default]
    Mock,
    #[cfg(feature = "llama-cpp")]
    LlamaCpp,
}

/// Configuration for the ML subsystem.
#[derive(Debug, Clone)]
pub struct MlConfig {
    /// Which provider to use.
    pub provider: ProviderType,
    /// Path to the GGUF model file (required for LlamaCpp provider).
    pub model_path: Option<PathBuf>,
    /// Context window size in tokens.
    pub n_ctx: u32,
    /// Number of CPU threads for inference.
    pub n_threads: u32,
    /// Number of GPU layers to offload (0 = CPU only).
    pub n_gpu_layers: u32,
}

impl Default for MlConfig {
    fn default() -> Self {
        Self {
            provider: ProviderType::default(),
            model_path: None,
            n_ctx: 2048,
            n_threads: 4,
            n_gpu_layers: 0,
        }
    }
}

// ── LlamaCpp-specific configuration ────────────────────────────────────

#[cfg(feature = "llama-cpp")]
mod llama_cpp_config {
    use super::*;

    /// Configuration specific to the LlamaCpp provider.
    #[derive(Debug, Clone)]
    pub struct LlamaCppConfig {
        /// Path to the GGUF model file.
        pub model_path: PathBuf,
        /// Context window size in tokens.
        pub n_ctx: u32,
        /// Number of CPU threads for inference.
        pub n_threads: u32,
        /// Number of GPU layers to offload (0 = CPU only).
        pub n_gpu_layers: u32,
    }

    impl Default for LlamaCppConfig {
        fn default() -> Self {
            Self {
                model_path: PathBuf::from("model.gguf"),
                n_ctx: 2048,
                n_threads: 4,
                n_gpu_layers: 0,
            }
        }
    }

    impl From<LlamaCppConfig> for MlConfig {
        fn from(cfg: LlamaCppConfig) -> Self {
            MlConfig {
                provider: ProviderType::LlamaCpp,
                model_path: Some(cfg.model_path.clone()),
                n_ctx: cfg.n_ctx,
                n_threads: cfg.n_threads,
                n_gpu_layers: cfg.n_gpu_layers,
            }
        }
    }
}

#[cfg(feature = "llama-cpp")]
pub use llama_cpp_config::LlamaCppConfig;