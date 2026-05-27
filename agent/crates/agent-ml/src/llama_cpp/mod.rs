mod provider;
mod prompt;

pub use provider::{LlamaCppConfig, LlamaCppProvider};
pub use prompt::{build_classification_prompt, parse_classification_output};