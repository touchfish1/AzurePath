use crate::core::ping;
use crate::types::monitor::{MonitorTarget, MonitorUpdatePayload, PingRecord};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::Emitter;

// ──────────────────────────────────────────────
// Database store
// ──────────────────────────────────────────────

#[derive(Debug)]
pub struct MonitorStore {
    conn: Mutex<Connection>,
}

impl MonitorStore {
    pub fn new() -> Result<Self, String> {
        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create monitor db directory: {}", e))?;
        }
        let conn =
            Connection::open(&db_path).map_err(|e| format!("Failed to open monitor db: {}", e))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_tables()?;
        Ok(store)
    }

    #[cfg(test)]
    pub fn new_test() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to create in-memory db: {}", e))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_tables()?;
        Ok(store)
    }

    fn db_path() -> Result<PathBuf, String> {
        let home = crate::core::utils::home_dir()
            .ok_or_else(|| "Cannot find home directory".to_string())?;
        Ok(home.join("AzurePath").join("monitor.db"))
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS monitor_targets (
                id TEXT PRIMARY KEY,
                host TEXT NOT NULL,
                label TEXT NOT NULL,
                interval_secs INTEGER NOT NULL DEFAULT 300,
                enabled INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE IF NOT EXISTS ping_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                target_id TEXT NOT NULL,
                target_host TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                latency_ms REAL,
                loss_rate REAL NOT NULL DEFAULT 0.0
            );
            CREATE INDEX IF NOT EXISTS idx_ping_history_target ON ping_history(target_id);
            CREATE INDEX IF NOT EXISTS idx_ping_history_ts ON ping_history(timestamp);
            ",
        )
        .map_err(|e| format!("Failed to init monitor tables: {}", e))?;
        Ok(())
    }

    // ── Target CRUD ──────────────────────────────────────────────

    pub fn add_target(&self, host: &str, label: &str, interval_secs: u64) -> Result<MonitorTarget, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO monitor_targets (id, host, label, interval_secs, enabled) VALUES (?1, ?2, ?3, ?4, 1)",
            rusqlite::params![id, host, label, interval_secs],
        )
        .map_err(|e| format!("Failed to insert target: {}", e))?;
        Ok(MonitorTarget {
            id,
            host: host.to_string(),
            label: label.to_string(),
            interval_secs,
            enabled: true,
        })
    }

    pub fn list_targets(&self) -> Result<Vec<MonitorTarget>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, host, label, interval_secs, enabled FROM monitor_targets ORDER BY label")
            .map_err(|e| format!("Failed to prepare: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(MonitorTarget {
                    id: row.get(0)?,
                    host: row.get(1)?,
                    label: row.get(2)?,
                    interval_secs: row.get::<_, i64>(3)? as u64,
                    enabled: row.get::<_, i32>(4)? != 0,
                })
            })
            .map_err(|e| format!("Failed to query targets: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read target row: {}", e))?;
        Ok(rows)
    }

    pub fn get_enabled_targets(&self) -> Result<Vec<MonitorTarget>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, host, label, interval_secs, enabled FROM monitor_targets WHERE enabled = 1 ORDER BY label",
            )
            .map_err(|e| format!("Failed to prepare: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(MonitorTarget {
                    id: row.get(0)?,
                    host: row.get(1)?,
                    label: row.get(2)?,
                    interval_secs: row.get::<_, i64>(3)? as u64,
                    enabled: row.get::<_, i32>(4)? != 0,
                })
            })
            .map_err(|e| format!("Failed to query enabled targets: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read target row: {}", e))?;
        Ok(rows)
    }

    pub fn delete_target(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM monitor_targets WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| format!("Failed to delete target: {}", e))?;
        conn.execute(
            "DELETE FROM ping_history WHERE target_id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| format!("Failed to delete target history: {}", e))?;
        Ok(())
    }

    // ── History ──────────────────────────────────────────────────

    pub fn insert_record(
        &self,
        target_id: &str,
        target_host: &str,
        timestamp: &str,
        latency_ms: Option<f64>,
        loss_rate: f64,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO ping_history (target_id, target_host, timestamp, latency_ms, loss_rate) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![target_id, target_host, timestamp, latency_ms, loss_rate],
        )
        .map_err(|e| format!("Failed to insert ping record: {}", e))?;
        Ok(())
    }

    pub fn get_history(&self, target_id: &str, since_days: i64) -> Result<Vec<PingRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let since = chrono::Utc::now() - chrono::Duration::days(since_days);
        let since_str = since.to_rfc3339();
        let mut stmt = conn
            .prepare(
                "SELECT id, target_id, target_host, timestamp, latency_ms, loss_rate
                 FROM ping_history
                 WHERE target_id = ?1 AND timestamp >= ?2
                 ORDER BY timestamp ASC",
            )
            .map_err(|e| format!("Failed to prepare: {}", e))?;
        let rows = stmt
            .query_map(rusqlite::params![target_id, since_str], |row| {
                Ok(PingRecord {
                    id: row.get(0)?,
                    target_id: row.get(1)?,
                    target_host: row.get(2)?,
                    timestamp: row.get(3)?,
                    latency_ms: row.get(4)?,
                    loss_rate: row.get(5)?,
                })
            })
            .map_err(|e| format!("Failed to query history: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read history row: {}", e))?;
        Ok(rows)
    }

    pub fn get_all_recent_history(&self, since_days: i64) -> Result<Vec<PingRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let since = chrono::Utc::now() - chrono::Duration::days(since_days);
        let since_str = since.to_rfc3339();
        let mut stmt = conn
            .prepare(
                "SELECT id, target_id, target_host, timestamp, latency_ms, loss_rate
                 FROM ping_history
                 WHERE timestamp >= ?2
                 ORDER BY timestamp ASC",
            )
            .map_err(|e| format!("Failed to prepare: {}", e))?;
        let rows = stmt
            .query_map(rusqlite::params![since_str], |row| {
                Ok(PingRecord {
                    id: row.get(0)?,
                    target_id: row.get(1)?,
                    target_host: row.get(2)?,
                    timestamp: row.get(3)?,
                    latency_ms: row.get(4)?,
                    loss_rate: row.get(5)?,
                })
            })
            .map_err(|e| format!("Failed to query all history: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read history row: {}", e))?;
        Ok(rows)
    }
}

// ── Monitoring control ───────────────────────────────────────────

static MONITOR_CANCEL: LazyLock<Mutex<Option<Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(None));
static MONITOR_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn is_monitoring() -> bool {
    MONITOR_ACTIVE.load(Ordering::Relaxed)
}

pub fn start_monitoring(app: tauri::AppHandle) -> Result<(), String> {
    // Stop any existing monitoring first
    stop_monitoring();

    let store = MonitorStore::new()?;
    let targets = store.get_enabled_targets()?;
    if targets.is_empty() {
        return Err("No enabled monitoring targets configured".to_string());
    }

    let cancel = Arc::new(AtomicBool::new(false));
    *MONITOR_CANCEL.lock().map_err(|e| e.to_string())? = Some(cancel.clone());
    MONITOR_ACTIVE.store(true, Ordering::Relaxed);

    // Emit initial status
    let _ = app.emit(
        "monitor:status",
        crate::types::monitor::MonitorStatusPayload {
            running: true,
            target_count: targets.len(),
        },
    );

    for target in targets {
        let cancel = cancel.clone();
        let app_clone = app.clone();
        let interval = std::time::Duration::from_secs(target.interval_secs);
        let target_id = target.id.clone();
        let target_host = target.host.clone();
        let target_label = target.label.clone();

        tokio::spawn(async move {
            // Do an initial ping immediately
            ping_target_and_report(
                &app_clone,
                &target_id,
                &target_host,
                &target_label,
            )
            .await;

            loop {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                tokio::time::sleep(interval).await;
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                ping_target_and_report(
                    &app_clone,
                    &target_id,
                    &target_host,
                    &target_label,
                )
                .await;
            }
        });
    }

    Ok(())
}

pub fn stop_monitoring() {
    if let Ok(mut guard) = MONITOR_CANCEL.lock() {
        if let Some(cancel) = guard.take() {
            cancel.store(true, Ordering::Relaxed);
        }
    }
    MONITOR_ACTIVE.store(false, Ordering::Relaxed);
}

async fn ping_target_and_report(
    app: &tauri::AppHandle,
    target_id: &str,
    target_host: &str,
    target_label: &str,
) {
    let result = ping::execute_ping(target_host, 3, 5000).await;
    match result {
        Ok(output) => {
            let results = ping::parse_ping_output(&output);
            let stats = ping::compute_stats(&results);

            let received = results.iter().filter(|r| r.status == "success").count() as u32;
            let loss_rate = if results.is_empty() {
                1.0
            } else {
                (results.len() as f64 - received as f64) / results.len() as f64
            };

            let avg_latency = if received > 0 { Some(stats.avg_ms) } else { None };

            let timestamp = chrono::Utc::now().to_rfc3339();

            // Store in database
            if let Ok(store) = MonitorStore::new() {
                let _ = store.insert_record(
                    target_id,
                    target_host,
                    &timestamp,
                    avg_latency,
                    loss_rate,
                );
            }

            // Emit update event
            let payload = MonitorUpdatePayload {
                target_id: target_id.to_string(),
                target_host: target_host.to_string(),
                label: target_label.to_string(),
                timestamp,
                latency_ms: avg_latency,
                loss_rate,
                min_ms: stats.min_ms,
                avg_ms: stats.avg_ms,
                max_ms: stats.max_ms,
                sent: stats.sent,
                received: stats.received,
            };
            let _ = app.emit("monitor:update", &payload);
        }
        Err(e) => {
            let timestamp = chrono::Utc::now().to_rfc3339();
            let ts_clone = timestamp.clone();
            let payload = MonitorUpdatePayload {
                target_id: target_id.to_string(),
                target_host: target_host.to_string(),
                label: target_label.to_string(),
                timestamp: ts_clone,
                latency_ms: None,
                loss_rate: 1.0,
                min_ms: 0.0,
                avg_ms: 0.0,
                max_ms: 0.0,
                sent: 3,
                received: 0,
            };
            let _ = app.emit("monitor:update", &payload);
            // Still store the failure
            if let Ok(store) = MonitorStore::new() {
                let _ = store.insert_record(target_id, target_host, &timestamp, None, 1.0);
            }
            tracing::warn!("Monitor ping failed for {}: {}", target_host, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> MonitorStore {
        MonitorStore::new_test().unwrap()
    }

    #[test]
    fn test_add_and_list_targets() {
        let store = make_store();
        let t = store.add_target("8.8.8.8", "Google DNS", 300).unwrap();
        assert_eq!(t.host, "8.8.8.8");
        assert_eq!(t.label, "Google DNS");
        assert!(t.enabled);

        let targets = store.list_targets().unwrap();
        assert_eq!(targets.len(), 1);
    }

    #[test]
    fn test_delete_target_removes_history() {
        let store = make_store();
        let t = store.add_target("1.1.1.1", "Cloudflare", 300).unwrap();
        store
            .insert_record(&t.id, &t.host, "2025-01-01T00:00:00Z", Some(10.0), 0.0)
            .unwrap();
        store.delete_target(&t.id).unwrap();
        assert!(store.list_targets().unwrap().is_empty());
        assert!(store.get_history(&t.id, 365).unwrap().is_empty());
    }

    #[test]
    fn test_insert_and_get_history() {
        let store = make_store();
        let t = store.add_target("8.8.4.4", "Google DNS2", 300).unwrap();
        store
            .insert_record(&t.id, &t.host, "2025-01-01T00:00:00Z", Some(10.0), 0.0)
            .unwrap();
        store
            .insert_record(&t.id, &t.host, "2025-01-01T00:01:00Z", Some(12.0), 0.0)
            .unwrap();

        let history = store.get_history(&t.id, 365).unwrap();
        assert_eq!(history.len(), 2);
        assert!((history[0].latency_ms.unwrap() - 10.0).abs() < 1e-9);
        assert!((history[1].latency_ms.unwrap() - 12.0).abs() < 1e-9);
    }

    #[test]
    fn test_get_history_with_time_filter() {
        let store = make_store();
        let t = store.add_target("test.local", "Test", 300).unwrap();
        // Insert a record with a timestamp far in the past
        store
            .insert_record(&t.id, &t.host, "2020-01-01T00:00:00Z", Some(5.0), 0.0)
            .unwrap();
        // This should be filtered out by since_days (which would be ~2000+ days ago)
        let history = store.get_history(&t.id, 1).unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_empty_store() {
        let store = make_store();
        assert!(store.list_targets().unwrap().is_empty());
        assert!(store.get_history("nonexistent", 7).unwrap().is_empty());
    }

    #[test]
    fn test_enabled_targets_only() {
        let store = make_store();
        store.add_target("8.8.8.8", "Google", 300).unwrap();
        // Disable the target by updating the DB directly
        let conn = store.conn.lock().unwrap();
        conn.execute(
            "UPDATE monitor_targets SET enabled = 0 WHERE label = 'Google'",
            [],
        )
        .unwrap();
        drop(conn);

        let enabled = store.get_enabled_targets().unwrap();
        assert!(enabled.is_empty());
    }

    #[test]
    fn test_insert_record_with_null_latency() {
        let store = make_store();
        let t = store.add_target("test.host", "Test", 300).unwrap();
        store
            .insert_record(&t.id, &t.host, "2025-06-01T00:00:00Z", None, 1.0)
            .unwrap();
        let history = store.get_history(&t.id, 365).unwrap();
        assert_eq!(history.len(), 1);
        assert!(history[0].latency_ms.is_none());
        assert!((history[0].loss_rate - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_is_monitoring_default_false() {
        assert!(!is_monitoring());
    }

    #[test]
    fn test_monitor_store_multiple_targets() {
        let store = make_store();
        store.add_target("8.8.8.8", "A", 60).unwrap();
        store.add_target("1.1.1.1", "B", 120).unwrap();
        store.add_target("192.168.1.1", "C", 300).unwrap();
        assert_eq!(store.list_targets().unwrap().len(), 3);
    }
}
