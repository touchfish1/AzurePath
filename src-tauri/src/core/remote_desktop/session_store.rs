//! Desktop session persistent storage using SQLite.

use rusqlite::{params, Connection};
use std::sync::Mutex;

use crate::types::remote_desktop::session::{DesktopSession, Protocol, SessionInput};

pub struct DesktopSessionStore {
    conn: Mutex<Connection>,
}

impl DesktopSessionStore {
    pub fn new() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_tables()?;
        Ok(store)
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS desktop_sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                protocol TEXT NOT NULL,
                host TEXT NOT NULL,
                port INTEGER NOT NULL,
                username TEXT NOT NULL,
                quality INTEGER NOT NULL DEFAULT 75,
                desktop_width INTEGER NOT NULL DEFAULT 1280,
                desktop_height INTEGER NOT NULL DEFAULT 720,
                domain TEXT DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS desktop_session_secrets (
                session_id TEXT PRIMARY KEY,
                password TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES desktop_sessions(id)
            );
            ",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Session CRUD ──

    pub fn list_sessions(&self) -> Result<Vec<DesktopSession>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, protocol, host, port, username, quality, \
                 desktop_width, desktop_height, domain, created_at, updated_at \
                 FROM desktop_sessions ORDER BY updated_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let sessions = stmt
            .query_map([], |row| {
                Ok(DesktopSession {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    protocol: match row.get::<_, String>(2)?.as_str() {
                        "rdp" => Protocol::Rdp,
                        _ => Protocol::Vnc,
                    },
                    host: row.get(3)?,
                    port: row.get(4)?,
                    username: row.get(5)?,
                    quality: row.get(6)?,
                    desktop_width: row.get(7)?,
                    desktop_height: row.get(8)?,
                    domain: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(sessions)
    }

    pub fn get_session(&self, id: &str) -> Result<Option<DesktopSession>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, protocol, host, port, username, quality, \
                 desktop_width, desktop_height, domain, created_at, updated_at \
                 FROM desktop_sessions WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![id], |row| {
                Ok(DesktopSession {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    protocol: match row.get::<_, String>(2)?.as_str() {
                        "rdp" => Protocol::Rdp,
                        _ => Protocol::Vnc,
                    },
                    host: row.get(3)?,
                    port: row.get(4)?,
                    username: row.get(5)?,
                    quality: row.get(6)?,
                    desktop_width: row.get(7)?,
                    desktop_height: row.get(8)?,
                    domain: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    pub fn create_session(&self, input: SessionInput) -> Result<DesktopSession, String> {
        let session = input.into_session();
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO desktop_sessions (id, name, protocol, host, port, username, quality, \
             desktop_width, desktop_height, domain, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                session.id,
                session.name,
                format!("{:?}", session.protocol).to_lowercase(),
                session.host,
                session.port,
                session.username,
                session.quality,
                session.desktop_width,
                session.desktop_height,
                session.domain,
                session.created_at,
                session.updated_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(session)
    }

    pub fn update_session(&self, id: &str, input: SessionInput) -> Result<DesktopSession, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE desktop_sessions SET name=?1, protocol=?2, host=?3, port=?4, username=?5, quality=?6, \
             desktop_width=?7, desktop_height=?8, domain=?9, updated_at=?10 WHERE id=?11",
            params![
                input.name,
                format!("{:?}", input.protocol).to_lowercase(),
                input.host,
                input.port,
                input.username,
                input.quality.unwrap_or(75),
                input.desktop_width.unwrap_or(1280),
                input.desktop_height.unwrap_or(720),
                input.domain,
                now,
                id,
            ],
        )
        .map_err(|e| e.to_string())?;
        drop(conn);
        self.get_session(id).map(|s| s.unwrap())
    }

    pub fn delete_session(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM desktop_session_secrets WHERE session_id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM desktop_sessions WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Secrets ──

    pub fn set_session_password(&self, session_id: &str, password: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO desktop_session_secrets (session_id, password) VALUES (?1, ?2)",
            params![session_id, password],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_session_password(&self, session_id: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT password FROM desktop_session_secrets WHERE session_id = ?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![session_id], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        Ok(rows.next().and_then(|r| r.ok()))
    }
}
