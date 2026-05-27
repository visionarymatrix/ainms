use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTab {
    pub title: String,
    pub url: String,
    pub r#type: String,
    pub browser: String,
    pub active: bool,
}

pub struct BrowserTabMonitor {
    client: reqwest::Client,
}

impl BrowserTabMonitor {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(3))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Poll all browser CDP endpoints for open tabs.
    /// Returns list of all open tabs across Chrome, Edge, and Brave.
    pub async fn get_all_tabs(&self) -> Vec<BrowserTab> {
        let mut all_tabs = Vec::new();

        let endpoints: Vec<(&str, u16)> = vec![
            ("chrome", 9222),
            ("msedge", 9229),
            ("brave", 9222),
        ];

        for (browser, port) in endpoints {
            match self.get_tabs_from_endpoint(browser, port).await {
                Ok(tabs) => {
                    if !tabs.is_empty() {
                        info!(browser, count = tabs.len(), "Browser tabs collected");
                    }
                    all_tabs.extend(tabs);
                }
                Err(_) => {
                    // Browser not running with debugging port - expected in most cases
                }
            }
        }

        all_tabs
    }

    async fn get_tabs_from_endpoint(&self, browser: &str, port: u16) -> Result<Vec<BrowserTab>, String> {
        let url = format!("http://127.0.0.1:{}/json", port);
        let resp = self.client.get(&url).send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("CDP endpoint returned {}", resp.status()));
        }

        let tabs: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;

        let browser_tabs: Vec<BrowserTab> = tabs
            .into_iter()
            .filter(|t| {
                let tab_type = t.get("type").and_then(|v| v.as_str()).unwrap_or("");
                tab_type == "page"
            })
            .map(|t| BrowserTab {
                title: t
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                url: t
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                r#type: "page".to_string(),
                browser: browser.to_string(),
                active: true,
            })
            .collect();

        Ok(browser_tabs)
    }
}