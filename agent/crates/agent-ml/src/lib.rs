use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub category: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClassificationType {
    Productive,
    Unproductive,
    Neutral,
    Unknown,
}

pub fn classify_app_title(_title: &str) -> ClassificationResult {
    // TODO: implement ML classification
    ClassificationResult {
        category: String::new(),
        confidence: 0.0,
    }
}