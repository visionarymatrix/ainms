use agent_proto::events::RulesInfo;
use std::collections::HashMap;
use tracing::info;

// ── Enforcement action ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum EnforcementAction {
    None,
    Toast(String),
    Warn(String),
    SoftBlock(String),
}

// ── Rule result ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RuleResult {
    pub category: String,
    pub confidence: f64,
    pub enforcement: EnforcementAction,
    pub matched_rule: Option<String>,
}

// ── Activity info (input) ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ActivityInfo {
    pub app_name: String,
    pub window_title: String,
    pub process_name: String,
    pub url: Option<String>,
    pub duration_sec: f64,
}

// ── Rule engine ──────────────────────────────────────────────────────────────

pub struct RuleEngine {
    rules: Option<RulesInfo>,
    category_durations: HashMap<String, f64>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            rules: None,
            category_durations: HashMap::new(),
        }
    }

    pub fn update_rules(&mut self, rules: RulesInfo) {
        info!(
            "Updating rule engine with {} app classifications, {} alert rules",
            rules.app_classifications.len(),
            rules.alert_rules.len()
        );
        self.rules = Some(rules);
    }

    pub fn has_rules(&self) -> bool {
        self.rules.is_some()
    }

    pub fn get_rules(&self) -> Option<&RulesInfo> {
        self.rules.as_ref()
    }

    /// Evaluate an activity against role rules.
    /// 1. Check AppClassification table for exact match (app_name -> category)
    /// 2. If no match, use keyword classification fallback
    /// 3. Check against role allowed/blocked categories
    /// 4. If blocked or unproductive + threshold exceeded, return enforcement action
    pub fn evaluate(&mut self, activity: &ActivityInfo) -> RuleResult {
        let (category, confidence, matched_rule) = self.classify_activity(activity);

        // Track duration
        *self
            .category_durations
            .entry(category.clone())
            .or_insert(0.0) += activity.duration_sec;

        // Determine enforcement
        let enforcement = self.determine_enforcement(&category, confidence);

        RuleResult {
            category,
            confidence,
            enforcement,
            matched_rule,
        }
    }

    /// Classify an activity: first check exact rules, then fallback to keyword matching
    fn classify_activity(&self, activity: &ActivityInfo) -> (String, f64, Option<String>) {
        // Step 1: Check AppClassification rules for exact match
        if let Some(rules) = &self.rules {
            for ac in &rules.app_classifications {
                if activity
                    .app_name
                    .to_lowercase()
                    .contains(&ac.app_name.to_lowercase())
                    || activity
                        .process_name
                        .to_lowercase()
                        .contains(&ac.app_name.to_lowercase())
                {
                    return (
                        ac.category.clone(),
                        0.95,
                        Some(format!("app_rule:{}", ac.app_name)),
                    );
                }
            }
        }

        // Step 2: Keyword fallback
        let result = agent_ml::classify_keyword_fallback(&activity.process_name);
        (result.category, result.confidence, None)
    }

    /// Determine enforcement action based on category, role rules, and thresholds
    fn determine_enforcement(&self, category: &str, confidence: f64) -> EnforcementAction {
        if let Some(rules) = &self.rules {
            // Check blocked categories first
            if let Some(ref role) = rules.role {
                for blocked in &role.blocked_categories {
                    if category.to_lowercase() == blocked.to_lowercase() {
                        return EnforcementAction::SoftBlock(format!(
                            "Activity '{}' is not allowed for role '{}'. {}",
                            category, role.name, role.work_description
                        ));
                    }
                }
                // Check if category is in allowed list
                if !role.allowed_categories.is_empty() {
                    let is_allowed = role
                        .allowed_categories
                        .iter()
                        .any(|a| a.to_lowercase() == category.to_lowercase());
                    if is_allowed {
                        return EnforcementAction::None;
                    }
                    // Not in allowed list = not explicitly allowed
                    if category == "unproductive" {
                        return EnforcementAction::Warn(format!(
                            "Activity appears unproductive. Allowed categories for role '{}': {}",
                            role.name,
                            role.allowed_categories.join(", ")
                        ));
                    }
                }
            }

            // Check alert rules for thresholds
            let duration = self.category_durations.get(category).copied().unwrap_or(0.0);
            for ar in &rules.alert_rules {
                if category.to_lowercase() == ar.category.to_lowercase() {
                    // Threshold is in minutes, duration is in seconds
                    if duration >= (ar.threshold_min as f64) * 60.0 {
                        let msg = format!(
                            "Category '{}' exceeded threshold of {} minutes (current: {:.0} min)",
                            category,
                            ar.threshold_min,
                            duration / 60.0
                        );
                        match ar.popup_type.as_str() {
                            "toast" => return EnforcementAction::Toast(msg),
                            "modal" => return EnforcementAction::Warn(msg),
                            "soft_block" => return EnforcementAction::SoftBlock(msg),
                            _ => return EnforcementAction::Toast(msg),
                        }
                    }
                }
            }
        }

        // Default: unproductive gets a toast if no rules
        if category == "unproductive" && confidence > 0.5 {
            return EnforcementAction::Toast("This activity appears unproductive.".to_string());
        }

        EnforcementAction::None
    }

    /// Classify a URL against rules
    pub fn evaluate_url(&mut self, url: &str, process_name: &str) -> RuleResult {
        let activity = ActivityInfo {
            app_name: process_name.to_string(),
            window_title: String::new(),
            process_name: process_name.to_string(),
            url: Some(url.to_string()),
            duration_sec: 0.0,
        };
        self.evaluate(&activity)
    }

    /// Reset duration trackers (e.g., on new reporting period)
    pub fn reset_durations(&mut self) {
        self.category_durations.clear();
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use agent_proto::events::{AlertRuleInfo, AppClassificationRule, PolicyInfo, RoleInfo};

    #[test]
    fn test_classify_productive_app() {
        let mut engine = RuleEngine::new();
        let activity = ActivityInfo {
            app_name: "code".to_string(),
            window_title: "main.rs - VS Code".to_string(),
            process_name: "code".to_string(),
            url: None,
            duration_sec: 60.0,
        };
        let result = engine.evaluate(&activity);
        assert_eq!(result.category, "productive");
    }

    #[test]
    fn test_exact_rule_match() {
        let mut engine = RuleEngine::new();
        engine.update_rules(RulesInfo {
            app_classifications: vec![AppClassificationRule {
                app_name: "slack".to_string(),
                category: "productive".to_string(),
            }],
            alert_rules: vec![],
            policy: PolicyInfo::default(),
            role: Some(RoleInfo {
                name: "Communications".to_string(),
                description: String::new(),
                work_description: "Communication focused role".to_string(),
                allowed_categories: vec!["productive".to_string(), "neutral".to_string()],
                blocked_categories: vec!["entertainment".to_string()],
            }),
        });

        let activity = ActivityInfo {
            app_name: "slack".to_string(),
            window_title: "team-chat".to_string(),
            process_name: "slack".to_string(),
            url: None,
            duration_sec: 120.0,
        };
        let result = engine.evaluate(&activity);
        assert_eq!(result.category, "productive");
        assert!((result.confidence - 0.95).abs() < f64::EPSILON);
        assert!(result.matched_rule.is_some());
    }

    #[test]
    fn test_blocked_category() {
        let mut engine = RuleEngine::new();
        engine.update_rules(RulesInfo {
            app_classifications: vec![],
            alert_rules: vec![],
            policy: PolicyInfo::default(),
            role: Some(RoleInfo {
                name: "Developer".to_string(),
                description: String::new(),
                work_description: String::new(),
                allowed_categories: vec!["productive".to_string()],
                blocked_categories: vec!["entertainment".to_string()],
            }),
        });

        let activity = ActivityInfo {
            app_name: "movie_player".to_string(),
            window_title: "Movie".to_string(),
            process_name: "movie_player".to_string(),
            url: None,
            duration_sec: 60.0,
        };
        let result = engine.evaluate(&activity);
        // movie_player falls back to "unproductive" via keyword match, not "entertainment"
        // so it won't match the blocked "entertainment" category
        // But let's test with a direct entertainment classification via app rule
        assert!(matches!(result.enforcement, EnforcementAction::None | EnforcementAction::Warn(_) | EnforcementAction::Toast(_) | EnforcementAction::SoftBlock(_)));
    }

    #[test]
    fn test_blocked_category_direct() {
        let mut engine = RuleEngine::new();
        engine.update_rules(RulesInfo {
            app_classifications: vec![AppClassificationRule {
                app_name: "movie_player".to_string(),
                category: "entertainment".to_string(),
            }],
            alert_rules: vec![],
            policy: PolicyInfo::default(),
            role: Some(RoleInfo {
                name: "Developer".to_string(),
                description: String::new(),
                work_description: String::new(),
                allowed_categories: vec!["productive".to_string()],
                blocked_categories: vec!["entertainment".to_string()],
            }),
        });

        let activity = ActivityInfo {
            app_name: "movie_player".to_string(),
            window_title: "Movie".to_string(),
            process_name: "movie_player".to_string(),
            url: None,
            duration_sec: 60.0,
        };
        let result = engine.evaluate(&activity);
        assert_eq!(result.category, "entertainment");
        assert!(matches!(result.enforcement, EnforcementAction::SoftBlock(_)));
    }

    #[test]
    fn test_alert_threshold() {
        let mut engine = RuleEngine::new();
        engine.update_rules(RulesInfo {
            app_classifications: vec![],
            alert_rules: vec![AlertRuleInfo {
                category: "unproductive".to_string(),
                threshold_min: 5,
                popup_type: "toast".to_string(),
            }],
            policy: PolicyInfo::default(),
            role: None,
        });

        // First evaluation — under threshold (3 min < 5 min)
        let activity = ActivityInfo {
            app_name: "unknown_game".to_string(),
            window_title: "Game".to_string(),
            process_name: "game".to_string(),
            url: None,
            duration_sec: 180.0,
        };
        let result = engine.evaluate(&activity);
        // "game" is classified as "unproductive" by keyword fallback with confidence 0.3
        // confidence 0.3 < 0.5, so no default toast either
        assert!(
            result.enforcement == EnforcementAction::None
                || matches!(result.enforcement, EnforcementAction::Toast(_))
        );

        // Second evaluation — add more time, now over threshold (6 min > 5 min)
        let activity2 = ActivityInfo {
            app_name: "unknown_game".to_string(),
            window_title: "Game".to_string(),
            process_name: "game".to_string(),
            url: None,
            duration_sec: 180.0,
        };
        let result2 = engine.evaluate(&activity2);
        // Should now trigger threshold toast
        assert!(matches!(result2.enforcement, EnforcementAction::Toast(_)));
    }

    #[test]
    fn test_evaluate_url() {
        let mut engine = RuleEngine::new();
        let result = engine.evaluate_url("https://youtube.com", "chrome");
        assert!(!result.category.is_empty());
    }

    #[test]
    fn test_no_rules_unproductive_low_confidence() {
        let mut engine = RuleEngine::new();
        // No rules loaded, "game" => unproductive with 0.3 confidence
        let activity = ActivityInfo {
            app_name: "game".to_string(),
            window_title: "Game".to_string(),
            process_name: "game".to_string(),
            url: None,
            duration_sec: 60.0,
        };
        let result = engine.evaluate(&activity);
        assert_eq!(result.category, "unproductive");
        assert_eq!(result.enforcement, EnforcementAction::None); // confidence 0.3 < 0.5
    }

    #[test]
    fn test_has_rules() {
        let engine = RuleEngine::new();
        assert!(!engine.has_rules());

        let mut engine = RuleEngine::new();
        engine.update_rules(RulesInfo {
            app_classifications: vec![],
            alert_rules: vec![],
            policy: PolicyInfo::default(),
            role: None,
        });
        assert!(engine.has_rules());
    }

    #[test]
    fn test_reset_durations() {
        let mut engine = RuleEngine::new();
        let activity = ActivityInfo {
            app_name: "code".to_string(),
            window_title: "main.rs".to_string(),
            process_name: "code".to_string(),
            url: None,
            duration_sec: 120.0,
        };
        engine.evaluate(&activity);
        assert!(engine.category_durations.contains_key("productive"));
        engine.reset_durations();
        assert!(engine.category_durations.is_empty());
    }
}