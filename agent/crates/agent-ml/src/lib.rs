//! # agent-ml
//!
//! Local LLM inference for on-device classification, backed by an
//! abstract [`LlmProvider`] trait so the underlying engine can be
//! swapped without changing call-sites.
//!
//! ## Feature flags
//!
//! - `llama-cpp` — enables the [`LlamaCppProvider`] backed by the
//!   `llama-cpp-2` crate (requires C/C++ toolchain at build time).
//!
//! ## Quick start (Mock provider — no model needed)
//!
//! ```rust
//! use agent_ml::provider::LlmProvider;
//! use agent_ml::mock::MockProvider;
//! use agent_ml::types::GenerateOptions;
//!
//! #[tokio::main]
//! async fn main() {
//!     let provider = MockProvider::new();
//!     let result = provider.generate("Hello", &GenerateOptions::default()).await;
//! }
//! ```

pub mod agent;
pub mod config;
pub mod error;
pub mod mock;
pub mod provider;
pub mod tool_search;
pub mod types;

#[cfg(feature = "llama-cpp")]
pub mod llama_cpp;

// ── Existing types (kept for backwards compat) ─────────────────────────

use serde::{Deserialize, Serialize};

/// Result of a classification prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub category: String,
    pub confidence: f64,
}

/// Category labels for activity classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClassificationType {
    Productive,
    Unproductive,
    Neutral,
    Unknown,
}

impl ClassificationType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "productive" => ClassificationType::Productive,
            "unproductive" => ClassificationType::Unproductive,
            "neutral" => ClassificationType::Neutral,
            _ => ClassificationType::Unknown,
        }
    }
}

/// Placeholder — will be replaced with LLM-powered classification.
pub fn classify_app_title(_title: &str) -> ClassificationResult {
    // TODO: wire to LlmProvider::classify() once agent-core integration is done
    ClassificationResult {
        category: String::new(),
        confidence: 0.0,
    }
}

/// Keyword-based classification fallback (moved from agent-core).
pub fn classify_keyword_fallback(name: &str) -> ClassificationResult {
    let lower = name.to_lowercase();
    let productive = [
        "ssh", "bash", "zsh", "vim", "nano", "code", "cargo", "go", "python",
        "node", "rustc", "rust-analyzer", "git", "make", "cmake", "docker",
        "kubectl", "terraform", "ansible", "java", "javac", "mvn", "gradle",
        "powershell", "cmd", "devenv", "msbuild", "dotnet",
    ];
    let neutral = [
        "firefox", "chrome", "chromium", "safari", "edge", "brave",
        "teams", "slack", "discord", "zoom", "thunderbird", "mail",
        "outlook", "explorer", "iexplore",
    ];

    for p in &productive {
        if lower.contains(p) {
            return ClassificationResult {
                category: "productive".to_string(),
                confidence: 0.85,
            };
        }
    }
    for n in &neutral {
        if lower.contains(n) {
            return ClassificationResult {
                category: "neutral".to_string(),
                confidence: 0.6,
            };
        }
    }
    ClassificationResult {
        category: "unproductive".to_string(),
        confidence: 0.3,
    }
}