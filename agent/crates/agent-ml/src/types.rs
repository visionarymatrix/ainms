/// Options for text generation.
#[derive(Debug, Clone)]
pub struct GenerateOptions {
    /// Maximum number of tokens to generate.
    pub max_tokens: usize,
    /// Sampling temperature (0.0 = greedy, higher = more random).
    pub temperature: f32,
    /// Top-p (nucleus) sampling threshold.
    pub top_p: f32,
    /// Top-k sampling threshold.
    pub top_k: i32,
    /// Repeat penalty factor.
    pub repeat_penalty: f32,
    /// Random seed for reproducible generation (None = random).
    pub seed: Option<u64>,
    /// Strings that stop generation when encountered.
    pub stop_strings: Vec<String>,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            max_tokens: 256,
            temperature: 0.8,
            top_p: 0.95,
            top_k: 40,
            repeat_penalty: 1.1,
            seed: Some(42),
            stop_strings: vec![],
        }
    }
}

impl GenerateOptions {
    /// Create options suitable for deterministic, low-variance outputs
    /// (e.g. classification, extraction).
    pub fn deterministic() -> Self {
        Self {
            temperature: 0.1,
            top_p: 0.9,
            top_k: 10,
            seed: Some(42),
            ..Default::default()
        }
    }
}