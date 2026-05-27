//! Prompt templates for LLM-based classification.

use crate::ClassificationResult;
use crate::error::{MlError, MlResult};

/// Build a classification prompt that asks the model to return structured JSON.
///
/// The prompt is designed for small models. It gives the model a list of
/// categories, the text to classify, and asks for a JSON response with
/// `category` and `confidence` fields.
pub fn build_classification_prompt(text: &str, categories: &[&str], context: Option<&str>) -> String {
    let cat_list = categories.join(", ");
    let context_line = context
        .map(|c| format!("\nContext: {}", c))
        .unwrap_or_default();

    format!(
        r#"Classify the following text into exactly one of these categories: {cat_list}.
Respond with ONLY a JSON object on a single line, no other text.
Format: {{"category": "<category>", "confidence": <0.0-1.0>}}{context_line}

Text: "{text}""#,
        cat_list = cat_list,
        context_line = context_line,
        text = text,
    )
}

/// Parse the raw model output into a [`ClassificationResult`].
///
/// Tries JSON first; falls back to extracting a category keyword from
/// plain text. Returns an error only when the output cannot be interpreted
/// at all.
pub fn parse_classification_output(raw: &str) -> MlResult<ClassificationResult> {
    let trimmed = raw.trim();

    // Attempt JSON parse — the model may have emitted extra text around it.
    let json_start = trimmed.find('{');
    let json_end = trimmed.rfind('}');
    if let (Some(s), Some(e)) = (json_start, json_end) {
        let json_str = &trimmed[s..=e];
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
            let category = v
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let confidence = v
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            return Ok(ClassificationResult {
                category: category.to_string(),
                confidence,
            });
        }
    }

    // Fallback: try to match a known category keyword in the text.
    let known = ["productive", "unproductive", "neutral"];
    let lower = trimmed.to_lowercase();
    for k in &known {
        if lower.contains(k) {
            return Ok(ClassificationResult {
                category: k.to_string(),
                confidence: 0.5,
            });
        }
    }

    Err(MlError::Generation(format!(
        "Could not parse classification output: {:?}",
        trimmed
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_classification_prompt_basic() {
        let prompt = build_classification_prompt("vim main.rs", &["productive", "neutral", "unproductive"], None);
        assert!(prompt.contains("productive"));
        assert!(prompt.contains("vim main.rs"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_build_classification_prompt_with_context() {
        let prompt = build_classification_prompt(
            "chrome",
            &["productive", "neutral"],
            Some("activity monitoring"),
        );
        assert!(prompt.contains("activity monitoring"));
    }

    #[test]
    fn test_parse_json_output() {
        let result = parse_classification_output(r#"{"category": "productive", "confidence": 0.92}"#).unwrap();
        assert_eq!(result.category, "productive");
        assert!((result.confidence - 0.92).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_json_with_surrounding_text() {
        let raw = r#"Sure! Here is the classification:
{"category": "neutral", "confidence": 0.75}
Hope that helps!"#;
        let result = parse_classification_output(raw).unwrap();
        assert_eq!(result.category, "neutral");
    }

    #[test]
    fn test_parse_fallback_keyword() {
        let result = parse_classification_output("The text is productive").unwrap();
        assert_eq!(result.category, "productive");
        assert_eq!(result.confidence, 0.5);
    }

    #[test]
    fn test_parse_failure() {
        let result = parse_classification_output("gibberish zzz");
        assert!(result.is_err());
    }
}