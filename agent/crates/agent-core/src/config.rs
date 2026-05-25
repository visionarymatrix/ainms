use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateFile {
    #[serde(default)]
    pub agent: AgentStateSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateSection {
    #[serde(default)]
    pub server: String,
    #[serde(default)]
    pub install_token: String,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub device_token: String,
}

impl Default for AgentStateFile {
    fn default() -> Self {
        Self {
            agent: AgentStateSection::default(),
        }
    }
}

impl Default for AgentStateSection {
    fn default() -> Self {
        Self {
            server: String::new(),
            install_token: String::new(),
            device_id: String::new(),
            device_token: String::new(),
        }
    }
}

pub fn default_config_path() -> String {
    if cfg!(target_os = "windows") {
        r"C:\ProgramData\AINMS\agent.conf".to_string()
    } else {
        "/etc/ainms/agent.conf".to_string()
    }
}

pub fn load_state(path: &str) -> Option<AgentStateFile> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            info!("No state file at {}: {}", path, e);
            return None;
        }
    };

    match toml::from_str(&contents) {
        Ok(state) => {
            info!("Loaded agent state from {}", path);
            Some(state)
        }
        Err(e) => {
            warn!("Failed to parse state file {}: {}", path, e);
            None
        }
    }
}

pub fn save_state(path: &str, state: &AgentStateFile) -> Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let contents = toml::to_string_pretty(state).context("Failed to serialize agent state")?;
    std::fs::write(path, &contents)
        .with_context(|| format!("Failed to write state file {}", path))?;
    info!("Saved agent state to {}", path);
    Ok(())
}

pub fn write_initial_config(path: &str, server: &str, install_token: &str) -> Result<()> {
    let state = AgentStateFile {
        agent: AgentStateSection {
            server: server.to_string(),
            install_token: install_token.to_string(),
            ..Default::default()
        },
    };
    save_state(path, &state)
}

#[allow(dead_code)]
pub fn load_config_from_file(path: &str) -> Result<crate::ConfigFile> {
    let contents = std::fs::read_to_string(path)?;
    let cfg: crate::ConfigFile = toml::from_str(&contents)?;
    Ok(cfg)
}