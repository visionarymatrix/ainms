use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use agent_proto::events::{AppUsageEventMeta, NetworkConnection, EmployeeInfo, ActivitySummary, DigitalProfileEntry};

// ── Schema versioning ────────────────────────────────────────────────────────

const SCHEMA_VERSION: u32 = 5;

const SCHEMA_UP: &str = r#"
    CREATE TABLE IF NOT EXISTS config (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS events (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        app_name        TEXT NOT NULL,
        window_title    TEXT NOT NULL DEFAULT '',
        process_name    TEXT NOT NULL DEFAULT '',
        process_id      INTEGER NOT NULL DEFAULT 0,
        start_time      TEXT NOT NULL,
        end_time        TEXT NOT NULL,
        duration_sec    REAL NOT NULL,
        classification  TEXT NOT NULL DEFAULT '',
        confidence      REAL NOT NULL DEFAULT 0.0,
        role_id         TEXT,
        device_id       TEXT NOT NULL,
        uploaded        INTEGER NOT NULL DEFAULT 0,
        created_at      REAL NOT NULL DEFAULT (strftime('%s','now'))
    );
    CREATE INDEX IF NOT EXISTS idx_events_uploaded ON events(uploaded);
    CREATE INDEX IF NOT EXISTS idx_events_created ON events(created_at);

    CREATE TABLE IF NOT EXISTS network_events (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        protocol        TEXT NOT NULL,
        local_ip        TEXT NOT NULL DEFAULT '',
        local_port      INTEGER NOT NULL DEFAULT 0,
        remote_ip       TEXT NOT NULL DEFAULT '',
        remote_port     INTEGER NOT NULL DEFAULT 0,
        state           TEXT NOT NULL DEFAULT '',
        process_id      INTEGER NOT NULL DEFAULT 0,
        process_name    TEXT NOT NULL DEFAULT '',
        remote_hostname TEXT,
        reconstructed_url TEXT,
        uploaded        INTEGER NOT NULL DEFAULT 0,
        created_at      REAL NOT NULL DEFAULT (strftime('%s','now'))
    );
    CREATE INDEX IF NOT EXISTS idx_net_uploaded ON network_events(uploaded);

    CREATE TABLE IF NOT EXISTS screenshots (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        image_data      BLOB NOT NULL,
        created_at      REAL NOT NULL DEFAULT (strftime('%s','now'))
    );
    CREATE INDEX IF NOT EXISTS idx_screenshots_created ON screenshots(created_at);
"#;

const SCHEMA_V3_UP: &str = r#"
    CREATE TABLE IF NOT EXISTS activity_summaries (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        device_id       TEXT NOT NULL,
        window_start    TEXT NOT NULL,
        window_end      TEXT NOT NULL,
        summary_text    TEXT NOT NULL,
        top_apps        TEXT NOT NULL DEFAULT '[]',
        screenshot_count INTEGER NOT NULL DEFAULT 0,
        uploaded        INTEGER NOT NULL DEFAULT 0,
        created_at      REAL NOT NULL DEFAULT (strftime('%s','now'))
    );
    CREATE INDEX IF NOT EXISTS idx_activity_uploaded ON activity_summaries(uploaded);
    CREATE INDEX IF NOT EXISTS idx_activity_created ON activity_summaries(created_at);
"#;

const SCHEMA_V4_UP: &str = r#"
    CREATE TABLE IF NOT EXISTS digital_profiles (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        app_name        TEXT NOT NULL,
        display_name    TEXT NOT NULL DEFAULT '',
        role_name        TEXT NOT NULL,
        category        TEXT NOT NULL,
        confidence      REAL NOT NULL DEFAULT 0.0,
        source          TEXT NOT NULL DEFAULT 'keyword_fallback',
        created_at      TEXT NOT NULL,
        updated_at      TEXT NOT NULL,
        UNIQUE(app_name, role_name)
    );
    CREATE INDEX IF NOT EXISTS idx_dp_role ON digital_profiles(role_name);
    CREATE INDEX IF NOT EXISTS idx_dp_category ON digital_profiles(category);
"#;

const SCHEMA_V5_UP: &str = r#"
    CREATE TABLE IF NOT EXISTS shown_alerts (
        alert_id  TEXT PRIMARY KEY,
        shown_at  REAL NOT NULL DEFAULT (strftime('%s','now'))
    );
    CREATE INDEX IF NOT EXISTS idx_shown_alerts_at ON shown_alerts(shown_at);
"#;

// ── Store struct ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

impl Store {
    /// Open (or create) the SQLite database at the given path.
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create database directory {}", parent.display()))?;
            }
        }

        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open database at {}", db_path.display()))?;

        // Production PRAGMAs for a desktop agent
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-2000;
             PRAGMA temp_store=MEMORY;",
        )
        .context("Failed to set database PRAGMAs")?;

        // Check schema version and create tables if needed
        let user_version: u32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap_or(0);

        if user_version < SCHEMA_VERSION {
            conn.execute_batch(SCHEMA_UP)
                .context("Failed to initialize database schema")?;
            if user_version < 3 {
                conn.execute_batch(SCHEMA_V3_UP)
                    .context("Failed to apply v3 schema migration (activity_summaries)")?;
            }
            if user_version < 4 {
                conn.execute_batch(SCHEMA_V4_UP)
                    .context("Failed to apply v4 schema migration (digital_profiles)")?;
            }
            if user_version < 5 {
                conn.execute_batch(SCHEMA_V5_UP)
                    .context("Failed to apply v5 schema migration (shown_alerts)")?;
            }
            conn.execute_batch(&format!("PRAGMA user_version = {};", SCHEMA_VERSION))
                .context("Failed to set schema version")?;
            info!(version = SCHEMA_VERSION, "Database schema initialized/updated");
        }

        info!(path = %db_path.display(), "Database opened");

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // ── Config / Settings ───────────────────────────────────────────────────

    /// Get a config value by key.
    pub fn get_config(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let val = conn
            .prepare_cached("SELECT value FROM config WHERE key = ?1")?
            .query_row(params![key], |row| row.get(0))
            .optional()?;
        Ok(val)
    }

    /// Set a config value (upsert).
    pub fn set_config(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        conn.prepare_cached("INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)")?
            .execute(params![key, value])?;
        Ok(())
    }

    /// Delete a config key.
    pub fn delete_config(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        conn.prepare_cached("DELETE FROM config WHERE key = ?1")?
            .execute(params![key])?;
        Ok(())
    }

    /// Get all config key-value pairs.
    pub fn get_all_config(&self) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let mut stmt = conn.prepare("SELECT key, value FROM config ORDER BY key")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // ── Agent State Persistence ──────────────────────────────────────────────

    /// Save agent identity state (device_id, device_token, install_token, server).
    pub fn save_agent_state(&self, server: &str, install_token: &str, device_id: &str, device_token: &str) -> Result<()> {
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let tx = conn.transaction()?;

        tx.prepare_cached("INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)")?
            .execute(params!["agent.server", server])?;
        tx.prepare_cached("INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)")?
            .execute(params!["agent.install_token", install_token])?;
        tx.prepare_cached("INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)")?
            .execute(params!["agent.device_id", device_id])?;
        tx.prepare_cached("INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)")?
            .execute(params!["agent.device_token", device_token])?;

        tx.commit()?;
        Ok(())
    }

    /// Load agent identity state from the config table.
    pub fn load_agent_state(&self) -> Result<AgentStateRecord> {
        Ok(AgentStateRecord {
            server: self.get_config("agent.server")?.unwrap_or_default(),
            install_token: self.get_config("agent.install_token")?.unwrap_or_default(),
            device_id: self.get_config("agent.device_id")?.unwrap_or_default(),
            device_token: self.get_config("agent.device_token")?.unwrap_or_default(),
        })
    }

    /// Save employee information to the config table.
    pub fn save_employee_info(&self, employee: &EmployeeInfo) -> Result<()> {
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let tx = conn.transaction()?;

        tx.prepare_cached("INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)")?
            .execute(params!["employee.id", employee.id])?;
        tx.prepare_cached("INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)")?
            .execute(params!["employee.company_id", employee.company_id])?;
        tx.prepare_cached("INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)")?
            .execute(params!["employee.employee_id", employee.employee_id])?;
        tx.prepare_cached("INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)")?
            .execute(params!["employee.name", employee.name])?;
        tx.prepare_cached("INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)")?
            .execute(params!["employee.email", employee.email])?;

        tx.commit()?;
        Ok(())
    }

    /// Load employee information from the config table.
    pub fn load_employee_info(&self) -> Result<Option<EmployeeInfo>> {
        let id = match self.get_config("employee.id")? {
            Some(id) if !id.is_empty() => id,
            _ => return Ok(None),
        };
        Ok(Some(EmployeeInfo {
            id,
            company_id: self.get_config("employee.company_id")?.unwrap_or_default(),
            employee_id: self.get_config("employee.employee_id")?.unwrap_or_default(),
            name: self.get_config("employee.name")?.unwrap_or_default(),
            email: self.get_config("employee.email")?.unwrap_or_default(),
        }))
    }

    /// Save a raw screenshot to the database and keep only the latest 5.
    pub fn save_screenshot(&self, data: &[u8]) -> Result<()> {
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let tx = conn.transaction()?;
        tx.prepare_cached("INSERT INTO screenshots (image_data) VALUES (?1)")?
            .execute(params![data])?;
        // Keep only the 5 most recent screenshots
        tx.execute(
            "DELETE FROM screenshots WHERE id NOT IN (
                SELECT id FROM screenshots ORDER BY created_at DESC LIMIT 5
            )",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Retrieve the latest screenshots (up to `limit` entries).
    pub fn get_latest_screenshots(&self, limit: usize) -> Result<Vec<Vec<u8>>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let mut stmt = conn.prepare_cached(
            "SELECT image_data FROM screenshots ORDER BY created_at DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| row.get(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // ── Events (AppUsageEventMeta) ───────────────────────────────────────────

    /// Insert an app usage event.
    pub fn insert_event(&self, event: &AppUsageEventMeta) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        conn.prepare_cached(
            "INSERT INTO events (app_name, window_title, process_name, process_id, start_time, end_time, duration_sec, classification, confidence, role_id, device_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
        )?
        .execute(params![
            event.app_name,
            event.window_title,
            event.process_name,
            event.process_id,
            event.start_time.to_rfc3339(),
            event.end_time.to_rfc3339(),
            event.duration_sec,
            event.classification,
            event.confidence,
            event.role_id,
            event.device_id,
        ])?;
        Ok(())
    }

    /// Insert multiple events in a transaction.
    pub fn insert_events(&self, events: &[AppUsageEventMeta]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO events (app_name, window_title, process_name, process_id, start_time, end_time, duration_sec, classification, confidence, role_id, device_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
            )?;
            for event in events {
                stmt.execute(params![
                    event.app_name,
                    event.window_title,
                    event.process_name,
                    event.process_id,
                    event.start_time.to_rfc3339(),
                    event.end_time.to_rfc3339(),
                    event.duration_sec,
                    event.classification,
                    event.confidence,
                    event.role_id,
                    event.device_id,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Get pending (not yet uploaded) events.
    pub fn get_pending_events(&self, limit: usize) -> Result<Vec<AppUsageEventMeta>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let mut stmt = conn.prepare_cached(
            "SELECT app_name, window_title, process_name, process_id, start_time, end_time, duration_sec, classification, confidence, role_id, device_id
             FROM events WHERE uploaded = 0 ORDER BY created_at ASC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let start_str: String = row.get(4)?;
            let end_str: String = row.get(5)?;
            let role_id: Option<String> = row.get(9)?;
            Ok(AppUsageEventMeta {
                app_name: row.get(0)?,
                window_title: row.get(1)?,
                process_name: row.get(2)?,
                process_id: row.get(3)?,
                start_time: start_str.parse().unwrap_or(chrono::Utc::now()),
                end_time: end_str.parse().unwrap_or(chrono::Utc::now()),
                duration_sec: row.get(6)?,
                classification: row.get(7)?,
                confidence: row.get(8)?,
                role_id,
                device_id: row.get(10)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Get pending event row IDs (for mark_uploaded).
    pub fn get_pending_event_ids(&self, limit: usize) -> Result<Vec<i64>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let mut stmt = conn.prepare_cached(
            "SELECT id FROM events WHERE uploaded = 0 ORDER BY created_at ASC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| row.get(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Mark events as uploaded (by row ID).
    pub fn mark_events_uploaded(&self, ids: &[i64]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let mut stmt = conn.prepare_cached("UPDATE events SET uploaded = 1 WHERE id = ?1")?;
        for id in ids {
            stmt.execute(params![id])?;
        }
        Ok(())
    }

    /// Purge old uploaded events (housekeeping).
    pub fn purge_uploaded_events(&self, older_than_secs: i64) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let cutoff = chrono::Utc::now().timestamp() - older_than_secs;
        let count = conn.execute("DELETE FROM events WHERE uploaded = 1 AND created_at < ?1", params![cutoff])?;
        Ok(count)
    }

    /// Count pending (unuploaded) events.
    pub fn pending_event_count(&self) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM events WHERE uploaded = 0", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    // ── Network Events ───────────────────────────────────────────────────────

    /// Insert network connections.
    pub fn insert_network_connections(&self, connections: &[NetworkConnection]) -> Result<()> {
        if connections.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO network_events (protocol, local_ip, local_port, remote_ip, remote_port, state, process_id, process_name, remote_hostname, reconstructed_url)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
            )?;
            for c in connections {
                stmt.execute(params![
                    c.protocol, c.local_ip, c.local_port, c.remote_ip, c.remote_port,
                    c.state, c.process_id, c.process_name, c.remote_hostname, c.reconstructed_url
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Get pending network connections.
    pub fn get_pending_network_connections(&self, limit: usize) -> Result<Vec<NetworkConnection>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let mut stmt = conn.prepare_cached(
            "SELECT protocol, local_ip, local_port, remote_ip, remote_port, state, process_id, process_name, remote_hostname, reconstructed_url
             FROM network_events WHERE uploaded = 0 ORDER BY created_at ASC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(NetworkConnection {
                protocol: row.get(0)?,
                local_ip: row.get(1)?,
                local_port: row.get(2)?,
                remote_ip: row.get(3)?,
                remote_port: row.get(4)?,
                state: row.get(5)?,
                process_id: row.get(6)?,
                process_name: row.get(7)?,
                remote_hostname: row.get(8)?,
                reconstructed_url: row.get(9)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Mark network events as uploaded by deleting them.
    pub fn mark_network_uploaded(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        conn.execute("DELETE FROM network_events WHERE uploaded = 0", [])?;
        Ok(())
    }

    /// Count pending network events.
    pub fn pending_network_count(&self) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM network_events WHERE uploaded = 0", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    // ── Activity Summaries ──────────────────────────────────────────────────

    /// Save an AI-generated activity summary.
    pub fn save_activity_summary(&self, summary: &ActivitySummary) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let top_apps_json = serde_json::to_string(&summary.top_apps).unwrap_or_else(|_| "[]".to_string());
        conn.prepare_cached(
            "INSERT INTO activity_summaries (device_id, window_start, window_end, summary_text, top_apps, screenshot_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )?
        .execute(params![
            summary.device_id,
            summary.window_start.to_rfc3339(),
            summary.window_end.to_rfc3339(),
            summary.summary_text,
            top_apps_json,
            summary.screenshot_count as i64,
        ])?;
        Ok(())
    }

    /// Get pending (not yet uploaded) activity summaries.
    pub fn get_pending_activity_summaries(&self, limit: usize) -> Result<Vec<ActivitySummary>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let mut stmt = conn.prepare_cached(
            "SELECT device_id, window_start, window_end, summary_text, top_apps, screenshot_count
             FROM activity_summaries WHERE uploaded = 0 ORDER BY created_at ASC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let window_start_str: String = row.get(1)?;
            let window_end_str: String = row.get(2)?;
            let top_apps_str: String = row.get(4)?;
            let top_apps: Vec<String> = serde_json::from_str(&top_apps_str).unwrap_or_default();
            let screenshot_count: i64 = row.get(5)?;
            Ok(ActivitySummary {
                device_id: row.get(0)?,
                timestamp: window_end_str.parse().unwrap_or_else(|_| chrono::Utc::now()),
                window_start: window_start_str.parse().unwrap_or_else(|_| chrono::Utc::now()),
                window_end: window_end_str.parse().unwrap_or_else(|_| chrono::Utc::now()),
                summary_text: row.get(3)?,
                top_apps,
                screenshot_count: screenshot_count as u32,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Get pending activity summary row IDs (for mark_uploaded).
    pub fn get_pending_activity_summary_ids(&self, limit: usize) -> Result<Vec<i64>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let mut stmt = conn.prepare_cached(
            "SELECT id FROM activity_summaries WHERE uploaded = 0 ORDER BY created_at ASC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| row.get(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Mark activity summaries as uploaded (by row IDs).
    pub fn mark_activity_summaries_uploaded(&self, ids: &[i64]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let mut stmt = conn.prepare_cached("UPDATE activity_summaries SET uploaded = 1 WHERE id = ?1")?;
        for id in ids {
            stmt.execute(params![id])?;
        }
        Ok(())
    }

    /// Purge old activity summaries (housekeeping).
    pub fn purge_old_activity_summaries(&self, older_than_secs: i64) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let cutoff = chrono::Utc::now().timestamp() - older_than_secs;
        let count = conn.execute("DELETE FROM activity_summaries WHERE created_at < ?1", params![cutoff])?;
        Ok(count)
    }

    // ── Digital Profiles ───────────────────────────────────────────────────

    pub fn save_digital_profiles(&self, entries: &[DigitalProfileEntry]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO digital_profiles (app_name, display_name, role_name, category, confidence, source, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
            )?;
            for entry in entries {
                stmt.execute(params![
                    entry.app_name,
                    entry.display_name.as_deref().unwrap_or(""),
                    entry.role_name,
                    entry.category,
                    entry.confidence,
                    entry.source,
                    entry.created_at,
                    entry.updated_at,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_digital_profiles(&self, role_name: &str) -> Result<Vec<DigitalProfileEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let mut stmt = conn.prepare_cached(
            "SELECT app_name, display_name, role_name, category, confidence, source, created_at, updated_at
             FROM digital_profiles WHERE role_name = ?1 ORDER BY category, app_name"
        )?;
        let rows = stmt.query_map(params![role_name], |row| {
            let display_name: String = row.get(1)?;
            Ok(DigitalProfileEntry {
                app_name: row.get(0)?,
                display_name: if display_name.is_empty() { None } else { Some(display_name) },
                role_name: row.get(2)?,
                category: row.get(3)?,
                confidence: row.get(4)?,
                source: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn load_all_digital_profiles(&self) -> Result<Vec<DigitalProfileEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let mut stmt = conn.prepare_cached(
            "SELECT app_name, display_name, role_name, category, confidence, source, created_at, updated_at
             FROM digital_profiles ORDER BY role_name, category, app_name"
        )?;
        let rows = stmt.query_map([], |row| {
            let display_name: String = row.get(1)?;
            Ok(DigitalProfileEntry {
                app_name: row.get(0)?,
                display_name: if display_name.is_empty() { None } else { Some(display_name) },
                role_name: row.get(2)?,
                category: row.get(3)?,
                confidence: row.get(4)?,
                source: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn delete_digital_profiles_for_role(&self, role_name: &str) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let count = conn.execute("DELETE FROM digital_profiles WHERE role_name = ?1", params![role_name])?;
        Ok(count)
    }

    // ── Shown Alerts (Compliance Dialog Deduplication) ───────────────────────

    /// Check whether a compliance alert has already been shown to the user.
    pub fn is_alert_shown(&self, alert_id: &str) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM shown_alerts WHERE alert_id = ?1",
            params![alert_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Mark a compliance alert as shown so the dialog is not displayed again.
    pub fn mark_alert_shown(&self, alert_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        conn.prepare_cached(
            "INSERT OR REPLACE INTO shown_alerts (alert_id, shown_at) VALUES (?1, strftime('%s','now'))"
        )?.execute(params![alert_id])?;
        Ok(())
    }

    // ── Async wrappers (spawn_blocking for tokio) ────────────────────────────

    pub async fn insert_event_async(&self, event: AppUsageEventMeta) -> Result<()> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.insert_event(&event)).await?
    }

    pub async fn insert_events_async(&self, events: Vec<AppUsageEventMeta>) -> Result<()> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.insert_events(&events)).await?
    }

    pub async fn get_pending_events_async(&self, limit: usize) -> Result<Vec<AppUsageEventMeta>> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.get_pending_events(limit)).await?
    }

    pub async fn mark_events_uploaded_async(&self, ids: Vec<i64>) -> Result<()> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.mark_events_uploaded(&ids)).await?
    }

    pub async fn get_config_async(&self, key: String) -> Result<Option<String>> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.get_config(&key)).await?
    }

    pub async fn set_config_async(&self, key: String, value: String) -> Result<()> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.set_config(&key, &value)).await?
    }

    pub async fn save_employee_info_async(&self, employee: EmployeeInfo) -> Result<()> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.save_employee_info(&employee)).await?
    }

    pub async fn load_employee_info_async(&self) -> Result<Option<EmployeeInfo>> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.load_employee_info()).await?
    }

    pub async fn save_screenshot_async(&self, data: Vec<u8>) -> Result<()> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.save_screenshot(&data)).await?
    }

    pub async fn get_latest_screenshots_async(&self, limit: usize) -> Result<Vec<Vec<u8>>> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.get_latest_screenshots(limit)).await?
    }

    pub async fn insert_network_connections_async(&self, connections: Vec<NetworkConnection>) -> Result<()> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.insert_network_connections(&connections)).await?
    }

    pub async fn get_pending_network_async(&self, limit: usize) -> Result<Vec<NetworkConnection>> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.get_pending_network_connections(limit)).await?
    }

    pub async fn mark_network_uploaded_async(&self) -> Result<()> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.mark_network_uploaded()).await?
    }

    pub async fn save_activity_summary_async(&self, summary: ActivitySummary) -> Result<()> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.save_activity_summary(&summary)).await?
    }

    pub async fn get_pending_activity_summaries_async(&self, limit: usize) -> Result<Vec<ActivitySummary>> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.get_pending_activity_summaries(limit)).await?
    }

    pub async fn mark_activity_summaries_uploaded_async(&self, ids: Vec<i64>) -> Result<()> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.mark_activity_summaries_uploaded(&ids)).await?
    }

    pub async fn purge_old_activity_summaries_async(&self, older_than_secs: i64) -> Result<usize> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.purge_old_activity_summaries(older_than_secs)).await?
    }

    pub async fn save_digital_profiles_async(&self, entries: Vec<DigitalProfileEntry>) -> Result<()> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.save_digital_profiles(&entries)).await?
    }

    pub async fn load_digital_profiles_async(&self, role_name: String) -> Result<Vec<DigitalProfileEntry>> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.load_digital_profiles(&role_name)).await?
    }

    pub async fn load_all_digital_profiles_async(&self) -> Result<Vec<DigitalProfileEntry>> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.load_all_digital_profiles()).await?
    }

    pub async fn delete_digital_profiles_for_role_async(&self, role_name: String) -> Result<usize> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.delete_digital_profiles_for_role(&role_name)).await?
    }

    pub async fn is_alert_shown_async(&self, alert_id: String) -> Result<bool> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.is_alert_shown(&alert_id)).await?
    }

    pub async fn mark_alert_shown_async(&self, alert_id: String) -> Result<()> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.mark_alert_shown(&alert_id)).await?
    }

    // ── Legacy compatibility ─────────────────────────────────────────────────

    /// Default constructor — opens or creates DB at the default platform path.
    pub fn new() -> Self {
        let db_path = default_db_path();
        match Self::open(std::path::Path::new(&db_path)) {
            Ok(store) => store,
            Err(e) => {
                warn!("Failed to open default database: {}. Using in-memory fallback.", e);
                let conn = Connection::open_in_memory().expect("in-memory SQLite must work");
                conn.execute_batch(SCHEMA_UP).expect("schema creation must work in-memory");
                Self { conn: Arc::new(Mutex::new(conn)) }
            }
        }
    }

    pub async fn get_pending_bulk(&self) -> Result<Vec<AppUsageEventMeta>> {
        self.get_pending_events(1000)
    }

    pub async fn get_pending_priority(&self) -> Result<Vec<AppUsageEventMeta>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("lock poisoned: {}", e))?;
        let mut stmt = conn.prepare_cached(
            "SELECT app_name, window_title, process_name, process_id, start_time, end_time, duration_sec, classification, confidence, role_id, device_id
             FROM events WHERE uploaded = 0 AND classification = 'unproductive' ORDER BY created_at ASC LIMIT 1000"
        )?;
        let rows = stmt.query_map([], |row| {
            let start_str: String = row.get(4)?;
            let end_str: String = row.get(5)?;
            let role_id: Option<String> = row.get(9)?;
            Ok(AppUsageEventMeta {
                app_name: row.get(0)?,
                window_title: row.get(1)?,
                process_name: row.get(2)?,
                process_id: row.get(3)?,
                start_time: start_str.parse().unwrap_or(chrono::Utc::now()),
                end_time: end_str.parse().unwrap_or(chrono::Utc::now()),
                duration_sec: row.get(6)?,
                classification: row.get(7)?,
                confidence: row.get(8)?,
                role_id,
                device_id: row.get(10)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub async fn mark_uploaded(&self, _ids: &[uuid::Uuid]) -> Result<()> {
        warn!("mark_uploaded with UUIDs is deprecated, use mark_events_uploaded with row IDs");
        Ok(())
    }
}

// ── Agent State Record ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateRecord {
    pub server: String,
    pub install_token: String,
    pub device_id: String,
    pub device_token: String,
}

impl Default for AgentStateRecord {
    fn default() -> Self {
        Self {
            server: String::new(),
            install_token: String::new(),
            device_id: String::new(),
            device_token: String::new(),
        }
    }
}

// ── Default DB path ─────────────────────────────────────────────────────────

pub fn default_db_path() -> String {
    if cfg!(target_os = "windows") {
        r"C:\ProgramData\AINMS\agent.db".to_string()
    } else {
        "/var/lib/ainms/agent.db".to_string()
    }
}