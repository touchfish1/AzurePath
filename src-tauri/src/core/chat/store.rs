use crate::types::chat::StoredMessage;
use crate::types::discovery::PeerInfo;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct ChatStore {
    conn: Mutex<Connection>,
}

impl ChatStore {
    pub fn new() -> Result<Self, String> {
        let db_path = Self::db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create db directory: {}", e))?;
        }

        let conn =
            Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_tables()?;
        Ok(store)
    }

    fn db_path() -> Result<PathBuf, String> {
        let home = home_dir().ok_or("Cannot find home directory")?;
        Ok(home.join("AzurePath").join("azurepath.db"))
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS messages (
                id          TEXT PRIMARY KEY,
                peer_id     TEXT NOT NULL,
                peer_name   TEXT NOT NULL,
                peer_ip     TEXT NOT NULL,
                peer_os     TEXT,
                content     TEXT NOT NULL,
                is_broadcast BOOLEAN DEFAULT false,
                is_incoming  BOOLEAN DEFAULT true,
                file_ref    TEXT,
                created_at  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS peers (
                id          TEXT PRIMARY KEY,
                hostname    TEXT NOT NULL,
                ip          TEXT NOT NULL,
                os          TEXT,
                last_seen   TEXT NOT NULL,
                status      TEXT DEFAULT 'online'
            );

            CREATE INDEX IF NOT EXISTS idx_messages_peer_id ON messages(peer_id);
            CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at);
            ",
        )
        .map_err(|e| format!("Failed to init tables: {}", e))?;
        Ok(())
    }

    pub fn insert_message(&self, msg: &StoredMessage) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO messages (id, peer_id, peer_name, peer_ip, peer_os, content, is_broadcast, is_incoming, file_ref, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                msg.id,
                msg.peer_id,
                msg.peer_name,
                msg.peer_ip,
                msg.peer_os,
                msg.content,
                msg.is_broadcast,
                msg.is_incoming,
                msg.file_ref,
                msg.created_at,
            ],
        )
        .map_err(|e| format!("Failed to insert message: {}", e))?;
        Ok(())
    }

    pub fn get_messages(&self, peer_id: Option<&str>, limit: u32) -> Result<Vec<StoredMessage>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let mut stmt = if peer_id.is_some() {
            conn.prepare(
                "SELECT id, peer_id, peer_name, peer_ip, peer_os, content, is_broadcast, is_incoming, file_ref, created_at
                 FROM messages WHERE peer_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            )
        } else {
            conn.prepare(
                "SELECT id, peer_id, peer_name, peer_ip, peer_os, content, is_broadcast, is_incoming, file_ref, created_at
                 FROM messages ORDER BY created_at DESC LIMIT ?1",
            )
        }
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = if let Some(pid) = peer_id {
            stmt.query_map(rusqlite::params![pid, limit], Self::map_row)
        } else {
            stmt.query_map(rusqlite::params![limit], Self::map_row)
        }
        .map_err(|e| format!("Failed to query messages: {}", e))?;

        let mut messages: Vec<StoredMessage> = Vec::new();
        for row in rows {
            messages.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
        }
        messages.reverse();
        Ok(messages)
    }

    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<StoredMessage> {
        Ok(StoredMessage {
            id: row.get(0)?,
            peer_id: row.get(1)?,
            peer_name: row.get(2)?,
            peer_ip: row.get(3)?,
            peer_os: row.get(4)?,
            content: row.get(5)?,
            is_broadcast: row.get(6)?,
            is_incoming: row.get(7)?,
            file_ref: row.get(8)?,
            created_at: row.get(9)?,
        })
    }

    pub fn upsert_peer(&self, peer: &PeerInfo) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO peers (id, hostname, ip, os, last_seen, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                hostname = excluded.hostname,
                ip = excluded.ip,
                os = excluded.os,
                last_seen = excluded.last_seen,
                status = excluded.status",
            rusqlite::params![
                peer.id,
                peer.hostname,
                peer.ip,
                peer.os,
                peer.last_seen,
                peer.status,
            ],
        )
        .map_err(|e| format!("Failed to upsert peer: {}", e))?;
        Ok(())
    }

    pub fn get_peers(&self) -> Result<Vec<PeerInfo>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, hostname, ip, os, last_seen, status FROM peers ORDER BY last_seen DESC",
            )
            .map_err(|e| format!("Failed to prepare peers query: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(PeerInfo {
                    id: row.get(0)?,
                    hostname: row.get(1)?,
                    ip: row.get(2)?,
                    os: row.get(3)?,
                    listen_port: crate::core::connection::LISTEN_PORT,
                    last_seen: row.get(4)?,
                    status: row.get(5)?,
                })
            })
            .map_err(|e| format!("Failed to query peers: {}", e))?;

        let mut peers = Vec::new();
        for row in rows {
            peers.push(row.map_err(|e| format!("Failed to read peer row: {}", e))?);
        }
        Ok(peers)
    }
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(std::path::PathBuf::from)
}
