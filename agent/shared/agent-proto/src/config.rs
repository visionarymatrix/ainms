use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub server_url: String,
    pub device_id: String,
    pub employee_id: String,
    pub upload_interval_secs: u64,
    pub heartbeat_interval_secs: u64,
    pub screenshot_policy: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        AgentConfig {
            server_url: String::from("http://localhost:8080"),
            device_id: String::new(),
            employee_id: String::new(),
            upload_interval_secs: 300,
            heartbeat_interval_secs: 60,
            screenshot_policy: String::from("on-demand"),
        }
    }
}