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

    pub fn search_messages(
        &self,
        keyword: &str,
        date_from: Option<&str>,
        date_to: Option<&str>,
    ) -> Result<Vec<StoredMessage>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if !keyword.is_empty() {
            let escaped = keyword
                .replace("\\", "\\\\")
                .replace("%", "\\%")
                .replace("_", "\\_");
            conditions.push(format!("(content LIKE ?{} ESCAPE '\\')", params.len() + 1));
            params.push(Box::new(format!("%{}%", escaped)));
        }
        if let Some(from) = date_from {
            conditions.push(format!("(created_at >= ?{})", params.len() + 1));
            params.push(Box::new(from.to_string()));
        }
        if let Some(to) = date_to {
            conditions.push(format!("(created_at <= ?{})", params.len() + 1));
            params.push(Box::new(to.to_string()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, peer_id, peer_name, peer_ip, peer_os, content, is_broadcast, is_incoming, file_ref, created_at
             FROM messages {}
             ORDER BY created_at DESC
             LIMIT 200",
            where_clause
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("Failed to prepare search query: {}", e))?;

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt
            .query_map(param_refs.as_slice(), Self::map_row)
            .map_err(|e| format!("Failed to query search: {}", e))?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
        }
        Ok(messages)
    }

    pub fn delete_messages(&self, ids: &[String]) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "DELETE FROM messages WHERE id IN ({})",
            placeholders.join(", ")
        );
        let params: Vec<&dyn rusqlite::types::ToSql> =
            ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        conn.execute(&sql, params.as_slice())
            .map_err(|e| format!("Failed to delete messages: {}", e))?;
        Ok(())
    }

    pub fn clear_history(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("DELETE FROM messages")
            .map_err(|e| format!("Failed to clear history: {}", e))?;
        Ok(())
    }

    #[cfg_attr(not(test), allow(dead_code))]
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

    #[cfg_attr(not(test), allow(dead_code))]
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

fn home_dir() -> Option<PathBuf> {
    crate::core::utils::home_dir()
}

#[cfg(test)]
impl ChatStore {
    /// Create a ChatStore backed by an in-memory SQLite database for testing.
    pub(crate) fn new_test() -> Result<Self, String> {
        let conn =
            Connection::open_in_memory().map_err(|e| format!("Failed to create in-memory db: {}", e))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_tables()?;
        Ok(store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_msg(id: &str, peer_id: &str, content: &str, created_at: &str) -> StoredMessage {
        StoredMessage {
            id: id.to_string(),
            peer_id: peer_id.to_string(),
            peer_name: peer_id.to_string(),
            peer_ip: "10.0.0.1".to_string(),
            peer_os: Some("linux".to_string()),
            content: content.to_string(),
            is_broadcast: false,
            is_incoming: true,
            file_ref: None,
            created_at: created_at.to_string(),
        }
    }

    fn msg_defaults(id: &str) -> StoredMessage {
        sample_msg(id, "peer-1", "Hello", "2025-01-01T00:00:00+00:00")
    }

    // -----------------------------------------------------------------------
    // Store construction
    // -----------------------------------------------------------------------

    #[test]
    fn test_new_test_store_is_empty() {
        let store = ChatStore::new_test().expect("in-memory store");
        let msgs = store.get_messages(None, 10).unwrap();
        assert!(msgs.is_empty());
    }

    // -----------------------------------------------------------------------
    // Insert and retrieve messages
    // -----------------------------------------------------------------------

    #[test]
    fn test_insert_and_retrieve_single_message() {
        let store = ChatStore::new_test().unwrap();
        let msg = msg_defaults("id-1");
        store.insert_message(&msg).unwrap();

        let all = store.get_messages(None, 10).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "id-1");
        assert_eq!(all[0].content, "Hello");
    }

    #[test]
    fn test_insert_and_retrieve_multiple_messages_in_order() {
        let store = ChatStore::new_test().unwrap();
        store
            .insert_message(&sample_msg("a", "p1", "first", "2025-01-01T00:00:01+00:00"))
            .unwrap();
        store
            .insert_message(&sample_msg("b", "p1", "second", "2025-01-01T00:00:02+00:00"))
            .unwrap();
        store
            .insert_message(&sample_msg("c", "p1", "third", "2025-01-01T00:00:03+00:00"))
            .unwrap();

        let all = store.get_messages(None, 10).unwrap();
        assert_eq!(all.len(), 3);
        // Should be in chronological order (ASC)
        assert_eq!(all[0].id, "a");
        assert_eq!(all[1].id, "b");
        assert_eq!(all[2].id, "c");
    }

    #[test]
    fn test_get_messages_by_peer_filter() {
        let store = ChatStore::new_test().unwrap();
        store
            .insert_message(&sample_msg("1", "alice", "hi", "2025-01-01T00:00:00+00:00"))
            .unwrap();
        store
            .insert_message(&sample_msg("2", "bob", "hey", "2025-01-01T00:00:01+00:00"))
            .unwrap();
        store
            .insert_message(&sample_msg("3", "alice", "how are you?", "2025-01-01T00:00:02+00:00"))
            .unwrap();

        let alice = store.get_messages(Some("alice"), 10).unwrap();
        assert_eq!(alice.len(), 2);
        for msg in &alice {
            assert_eq!(msg.peer_id, "alice");
        }
    }

    #[test]
    fn test_get_messages_limit() {
        let store = ChatStore::new_test().unwrap();
        for i in 0..20 {
            let ts = format!("2025-01-01T00:00:{:02}+00:00", i);
            store
                .insert_message(&sample_msg(&format!("m{}", i), "p1", &format!("msg {}", i), &ts))
                .unwrap();
        }

        // Limit should return only the N most recent messages (in chronological order)
        let limited = store.get_messages(None, 5).unwrap();
        assert_eq!(limited.len(), 5);
        assert_eq!(limited[0].id, "m15");
        assert_eq!(limited[1].id, "m16");
        assert_eq!(limited[2].id, "m17");
        assert_eq!(limited[3].id, "m18");
        assert_eq!(limited[4].id, "m19");
    }

    #[test]
    fn test_get_messages_limit_zero_returns_empty() {
        let store = ChatStore::new_test().unwrap();
        store.insert_message(&msg_defaults("m1")).unwrap();
        let result = store.get_messages(None, 0).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_get_messages_with_nonexistent_peer() {
        let store = ChatStore::new_test().unwrap();
        store.insert_message(&msg_defaults("m1")).unwrap();
        let result = store.get_messages(Some("nobody"), 10).unwrap();
        assert!(result.is_empty());
    }

    // -----------------------------------------------------------------------
    // Field preservation
    // -----------------------------------------------------------------------

    #[test]
    fn test_insert_preserves_all_fields() {
        let store = ChatStore::new_test().unwrap();
        let msg = StoredMessage {
            id: "full-test".to_string(),
            peer_id: "peer-42".to_string(),
            peer_name: "MyMachine".to_string(),
            peer_ip: "192.168.1.99".to_string(),
            peer_os: Some("windows".to_string()),
            content: "Check all fields!".to_string(),
            is_broadcast: true,
            is_incoming: false,
            file_ref: Some("/tmp/file.zip".to_string()),
            created_at: "2025-06-15T10:30:00+00:00".to_string(),
        };
        store.insert_message(&msg).unwrap();

        let msgs = store.get_messages(None, 10).unwrap();
        assert_eq!(msgs.len(), 1);
        let got = &msgs[0];
        assert_eq!(got.id, "full-test");
        assert_eq!(got.peer_id, "peer-42");
        assert_eq!(got.peer_name, "MyMachine");
        assert_eq!(got.peer_ip, "192.168.1.99");
        assert_eq!(got.peer_os, Some("windows".to_string()));
        assert_eq!(got.content, "Check all fields!");
        assert!(got.is_broadcast);
        assert!(!got.is_incoming);
        assert_eq!(got.file_ref, Some("/tmp/file.zip".to_string()));
        assert_eq!(got.created_at, "2025-06-15T10:30:00+00:00");
    }

    #[test]
    fn test_insert_with_null_optional_fields() {
        let store = ChatStore::new_test().unwrap();
        let msg = StoredMessage {
            id: "null-opts".to_string(),
            peer_id: "p1".to_string(),
            peer_name: "p1".to_string(),
            peer_ip: "10.0.0.1".to_string(),
            peer_os: None,
            content: "nulls test".to_string(),
            is_broadcast: false,
            is_incoming: true,
            file_ref: None,
            created_at: "2025-01-01T00:00:00+00:00".to_string(),
        };
        store.insert_message(&msg).unwrap();

        let msgs = store.get_messages(None, 10).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].peer_os, None);
        assert_eq!(msgs[0].file_ref, None);
    }

    // -----------------------------------------------------------------------
    // Error handling
    // -----------------------------------------------------------------------

    #[test]
    fn test_insert_duplicate_id_returns_error() {
        let store = ChatStore::new_test().unwrap();
        store.insert_message(&msg_defaults("dup-id")).unwrap();
        let err = store
            .insert_message(&msg_defaults("dup-id"))
            .expect_err("duplicate PK should fail");
        assert!(
            err.contains("UNIQUE") || err.contains("Failed to insert"),
            "Expected UNIQUE constraint error, got: {}",
            err
        );
    }

    // -----------------------------------------------------------------------
    // Content edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_insert_empty_content() {
        let store = ChatStore::new_test().unwrap();
        store
            .insert_message(&sample_msg("e1", "p1", "", "2025-01-01T00:00:00+00:00"))
            .unwrap();
        let msgs = store.get_messages(None, 10).unwrap();
        assert_eq!(msgs[0].content, "");
    }

    #[test]
    fn test_insert_unicode_content() {
        let store = ChatStore::new_test().unwrap();
        let unicode = "Hello 你好 Привет \u{1F600} \u{00E9}";
        store
            .insert_message(&sample_msg("u1", "p1", unicode, "2025-01-01T00:00:00+00:00"))
            .unwrap();
        let msgs = store.get_messages(None, 10).unwrap();
        assert_eq!(msgs[0].content, unicode);
    }

    #[test]
    fn test_insert_long_content() {
        let store = ChatStore::new_test().unwrap();
        let long = "A".repeat(100_000);
        store
            .insert_message(&sample_msg("long", "p1", &long, "2025-01-01T00:00:00+00:00"))
            .unwrap();
        let msgs = store.get_messages(None, 10).unwrap();
        assert_eq!(msgs[0].content.len(), 100_000);
    }

    // -----------------------------------------------------------------------
    // Peer operations
    // -----------------------------------------------------------------------

    fn make_peer(id: &str, hostname: &str, ip: &str, status: &str) -> PeerInfo {
        PeerInfo {
            id: id.to_string(),
            hostname: hostname.to_string(),
            ip: ip.to_string(),
            os: "linux".to_string(),
            listen_port: crate::core::connection::LISTEN_PORT,
            last_seen: "2025-01-01T00:00:00+00:00".to_string(),
            status: status.to_string(),
        }
    }

    #[test]
    fn test_upsert_and_get_peers() {
        let store = ChatStore::new_test().unwrap();
        store.upsert_peer(&make_peer("p1", "host1", "10.0.0.1", "online")).unwrap();
        store.upsert_peer(&make_peer("p2", "host2", "10.0.0.2", "online")).unwrap();

        let peers = store.get_peers().unwrap();
        assert_eq!(peers.len(), 2);
    }

    #[test]
    fn test_upsert_existing_peer_updates_fields() {
        let store = ChatStore::new_test().unwrap();
        store
            .upsert_peer(&make_peer("p1", "old-name", "10.0.0.1", "online"))
            .unwrap();
        store
            .upsert_peer(&make_peer("p1", "new-name", "10.0.0.2", "offline"))
            .unwrap();

        let peers = store.get_peers().unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].hostname, "new-name");
        assert_eq!(peers[0].ip, "10.0.0.2");
        assert_eq!(peers[0].status, "offline");
    }

    #[test]
    fn test_get_peers_empty() {
        let store = ChatStore::new_test().unwrap();
        let peers = store.get_peers().unwrap();
        assert!(peers.is_empty());
    }

    #[test]
    fn test_get_peers_ordered_by_last_seen_desc() {
        let store = ChatStore::new_test().unwrap();
        let old = PeerInfo {
            id: "old".to_string(),
            hostname: "old".to_string(),
            ip: "10.0.0.1".to_string(),
            os: "linux".to_string(),
            listen_port: 42070,
            last_seen: "2024-01-01T00:00:00+00:00".to_string(),
            status: "offline".to_string(),
        };
        let recent = PeerInfo {
            id: "recent".to_string(),
            hostname: "recent".to_string(),
            ip: "10.0.0.2".to_string(),
            os: "linux".to_string(),
            listen_port: 42070,
            last_seen: "2025-06-01T00:00:00+00:00".to_string(),
            status: "online".to_string(),
        };
        store.upsert_peer(&old).unwrap();
        store.upsert_peer(&recent).unwrap();

        let peers = store.get_peers().unwrap();
        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].id, "recent");
        assert_eq!(peers[1].id, "old");
    }

    // -----------------------------------------------------------------------
    // home_dir utility
    // -----------------------------------------------------------------------

    #[test]
    fn test_home_dir_returns_some() {
        // At least one of USERPROFILE or HOME should be set in any test environment
        let dir = home_dir();
        assert!(dir.is_some(), "Expected home_dir() to return a path");
        let path = dir.unwrap();
        assert!(path.is_absolute(), "Home directory should be absolute");
    }
}
