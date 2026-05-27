use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEnrollRequest {
    pub install_token: String,
    pub hostname: String,
    pub os_type: String,
    pub os_version: String,
    pub fingerprint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram_info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_addresses: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_addresses: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentRequest {
    pub employee_id: String,
    pub company_id: String,
    pub hostname: String,
    pub os_type: String,
    pub os_version: String,
    pub fingerprint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram_info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_addresses: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_addresses: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeInfo {
    pub id: String,
    pub company_id: String,
    pub employee_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppClassificationRule {
    pub app_name: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRuleInfo {
    pub category: String,
    pub threshold_min: i32,
    pub popup_type: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyInfo {
    #[serde(default)]
    pub upload_interval: i32,
    #[serde(default)]
    pub screenshot_enabled: bool,
    #[serde(default)]
    pub screenshot_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub work_description: String,
    #[serde(default)]
    pub allowed_categories: Vec<String>,
    #[serde(default)]
    pub blocked_categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesInfo {
    #[serde(default)]
    pub app_classifications: Vec<AppClassificationRule>,
    #[serde(default)]
    pub alert_rules: Vec<AlertRuleInfo>,
    #[serde(default)]
    pub policy: PolicyInfo,
    #[serde(default)]
    pub role: Option<RoleInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentResponse {
    pub device_id: String,
    pub employee_id: String,
    #[serde(default)]
    pub employee: Option<EmployeeInfo>,
    pub device_token: String,
    #[serde(default)]
    pub rules: Option<RulesInfo>,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUsageEventMeta {
    pub app_name: String,
    pub window_title: String,
    pub process_name: String,
    pub process_id: i32,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_sec: f64,
    pub classification: String,
    pub confidence: f64,
    pub role_id: Option<String>,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUsageSummary {
    pub device_id: String,
    pub app_name: String,
    pub total_duration_sec: f64,
    pub session_count: u64,
    pub productive_duration_sec: f64,
    pub unproductive_duration_sec: f64,
    pub neutral_duration_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkEventRequest {
    pub device_id: String,
    pub summary: AppUsageSummary,
    pub metadata: Vec<AppUsageEventMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityEventRequest {
    pub device_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
}

// Legacy types kept for compatibility with agent-comms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppUsageEvent {
    pub app_name: String,
    pub window_title: String,
    pub process_name: String,
    pub process_id: u32,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_sec: f64,
    pub classification: String,
    pub confidence: f64,
    pub device_id: String,
    pub employee_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupEvent {
    pub explanation: String,
    pub popup_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityEvent {
    pub event_type: String,
    pub payload: serde_json::Value,
    pub device_id: String,
    pub employee_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotResult {
    pub device_id: String,
    pub employee_id: String,
    pub timestamp: DateTime<Utc>,
    pub file_path: String,
    pub classification: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TamperEvent {
    pub device_id: String,
    pub employee_id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlePeriod {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSession {
    pub app_name: String,
    pub window_title: String,
    pub process_name: String,
    pub process_id: i32,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_sec: f64,
    pub idle_during: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingCommand {
    pub id: String,
    pub device_id: String,
    pub command_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub created_at: String,
}

// ── Network traffic monitoring events ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConnection {
    pub protocol: String,
    pub local_ip: String,
    pub local_port: u16,
    pub remote_ip: String,
    pub remote_port: u16,
    pub state: String,
    // 0 if unknown
    pub process_id: i32,
    // empty string if unknown
    pub process_name: String,
    pub remote_hostname: Option<String>,
    pub reconstructed_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTrafficEvent {
    pub device_id: String,
    pub timestamp: DateTime<Utc>,
    pub connections: Vec<NetworkConnection>,
    pub unresolved_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTrafficSummary {
    pub device_id: String,
    pub total_connections: u32,
    pub resolved_connections: u32,
    pub unique_domains: Vec<String>,
    // e.g. {"tcp": N, "udp": M}
    pub protocol_breakdown: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkNetworkEventRequest {
    pub device_id: String,
    pub summary: NetworkTrafficSummary,
    pub connections: Vec<NetworkConnection>,
}

// ── Browser tab monitoring events ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTabEvent {
    pub device_id: String,
    pub timestamp: DateTime<Utc>,
    pub tabs: Vec<BrowserTabInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTabInfo {
    pub title: String,
    pub url: String,
    pub browser: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkBrowserTabRequest {
    pub device_id: String,
    pub tabs: Vec<BrowserTabInfo>,
}