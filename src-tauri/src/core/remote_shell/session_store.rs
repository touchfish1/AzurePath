//! 会话持久化存储：使用 SQLite 存储会话配置和密码。

use rusqlite::{params, Connection};
use std::sync::Mutex;
use uuid::Uuid;

use crate::types::remote_shell::database::{DbConnection, DbConnectionInput, DbType};
use crate::types::remote_shell::session::{Protocol, RemoteSession, SessionInput};

pub struct SessionStore {
    conn: Mutex<Connection>,
}

impl SessionStore {
    pub fn new() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        let store = Self { conn: Mutex::new(conn) };
        store.init_tables()?;
        Ok(store)
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS remote_sessions (
                id TEXT PRIMARY KEY,
                environment TEXT NOT NULL DEFAULT 'default',
                name TEXT NOT NULL,
                protocol TEXT NOT NULL,
                host TEXT NOT NULL,
                port INTEGER NOT NULL,
                username TEXT NOT NULL,
                encoding TEXT NOT NULL DEFAULT 'utf-8',
                keepalive_secs INTEGER NOT NULL DEFAULT 30,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS remote_session_secrets (
                session_id TEXT PRIMARY KEY,
                password TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES remote_sessions(id)
            );
            CREATE TABLE IF NOT EXISTS db_connections (
                id TEXT PRIMARY KEY,
                environment TEXT NOT NULL DEFAULT 'default',
                name TEXT NOT NULL,
                db_type TEXT NOT NULL,
                host TEXT NOT NULL,
                port INTEGER NOT NULL,
                username TEXT NOT NULL,
                default_database TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS db_connection_secrets (
                connection_id TEXT PRIMARY KEY,
                password TEXT NOT NULL,
                FOREIGN KEY (connection_id) REFERENCES db_connections(id)
            );
            CREATE TABLE IF NOT EXISTS environments (
                name TEXT PRIMARY KEY
            );
            INSERT OR IGNORE INTO environments (name) VALUES ('default');
            ",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Session CRUD ──

    pub fn list_sessions(&self) -> Result<Vec<RemoteSession>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, environment, name, protocol, host, port, username, encoding, keepalive_secs, created_at, updated_at FROM remote_sessions ORDER BY updated_at DESC")
            .map_err(|e| e.to_string())?;
        let sessions = stmt
            .query_map([], |row| {
                Ok(RemoteSession {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    environment: row.get(1)?,
                    name: row.get(2)?,
                    protocol: match row.get::<_, String>(3)?.as_str() {
                        "telnet" => Protocol::Telnet,
                        _ => Protocol::Ssh,
                    },
                    host: row.get(4)?,
                    port: row.get(5)?,
                    username: row.get(6)?,
                    encoding: row.get(7)?,
                    keepalive_secs: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(sessions)
    }

    pub fn get_session(&self, id: &Uuid) -> Result<Option<RemoteSession>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, environment, name, protocol, host, port, username, encoding, keepalive_secs, created_at, updated_at FROM remote_sessions WHERE id = ?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![id.to_string()], |row| {
                Ok(RemoteSession {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    environment: row.get(1)?,
                    name: row.get(2)?,
                    protocol: match row.get::<_, String>(3)?.as_str() {
                        "telnet" => Protocol::Telnet,
                        _ => Protocol::Ssh,
                    },
                    host: row.get(4)?,
                    port: row.get(5)?,
                    username: row.get(6)?,
                    encoding: row.get(7)?,
                    keepalive_secs: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    pub fn create_session(&self, input: SessionInput) -> Result<RemoteSession, String> {
        let session = input.into_session();
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO remote_sessions (id, environment, name, protocol, host, port, username, encoding, keepalive_secs, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                session.id.to_string(),
                session.environment,
                session.name,
                format!("{:?}", session.protocol).to_lowercase(),
                session.host,
                session.port,
                session.username,
                session.encoding,
                session.keepalive_secs,
                session.created_at,
                session.updated_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(session)
    }

    pub fn update_session(&self, id: &Uuid, input: SessionInput) -> Result<RemoteSession, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE remote_sessions SET name=?1, protocol=?2, host=?3, port=?4, username=?5, encoding=?6, keepalive_secs=?7, updated_at=?8, environment=?9 WHERE id=?10",
            params![
                input.name,
                format!("{:?}", input.protocol).to_lowercase(),
                input.host,
                input.port,
                input.username,
                input.encoding.unwrap_or_else(|| "utf-8".to_string()),
                input.keepalive_secs.unwrap_or(30),
                now,
                input.environment.unwrap_or_else(|| "default".to_string()),
                id.to_string(),
            ],
        )
        .map_err(|e| e.to_string())?;
        drop(conn);
        self.get_session(id).map(|s| s.unwrap())
    }

    pub fn delete_session(&self, id: &Uuid) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM remote_session_secrets WHERE session_id = ?1", params![id.to_string()])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM remote_sessions WHERE id = ?1", params![id.to_string()])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Secrets ──

    pub fn set_session_password(&self, session_id: &Uuid, password: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO remote_session_secrets (session_id, password) VALUES (?1, ?2)",
            params![session_id.to_string(), password],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_session_password(&self, session_id: &Uuid) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT password FROM remote_session_secrets WHERE session_id = ?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![session_id.to_string()], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    // ── Environment ──

    pub fn list_environments(&self) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT name FROM environments ORDER BY name").map_err(|e| e.to_string())?;
        let envs = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(envs)
    }

    pub fn create_environment(&self, name: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("INSERT OR IGNORE INTO environments (name) VALUES (?1)", params![name])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── DB Connections ──

    pub fn list_db_connections(&self, db_type: Option<&str>) -> Result<Vec<DbConnection>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        if let Some(t) = db_type {
            let mut stmt = conn
                .prepare("SELECT id, environment, name, db_type, host, port, username, default_database, created_at, updated_at FROM db_connections WHERE db_type = ?1 ORDER BY updated_at DESC")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(rusqlite::params![t], |row| {
                    Ok(DbConnection {
                        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                        environment: row.get(1)?,
                        name: row.get(2)?,
                        db_type: match row.get::<_, String>(3)?.as_str() {
                            "postgresql" => DbType::Postgresql,
                            "redis" => DbType::Redis,
                            "zookeeper" => DbType::Zookeeper,
                            "etcd" => DbType::Etcd,
                            _ => DbType::Mysql,
                        },
                        host: row.get(4)?,
                        port: row.get(5)?,
                        username: row.get(6)?,
                        default_database: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        } else {
            let mut stmt = conn
                .prepare("SELECT id, environment, name, db_type, host, port, username, default_database, created_at, updated_at FROM db_connections ORDER BY updated_at DESC")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(DbConnection {
                        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                        environment: row.get(1)?,
                        name: row.get(2)?,
                        db_type: match row.get::<_, String>(3)?.as_str() {
                            "postgresql" => DbType::Postgresql,
                            "redis" => DbType::Redis,
                            "zookeeper" => DbType::Zookeeper,
                            "etcd" => DbType::Etcd,
                            _ => DbType::Mysql,
                        },
                        host: row.get(4)?,
                        port: row.get(5)?,
                        username: row.get(6)?,
                        default_database: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        }
    }

    pub fn create_db_connection(&self, input: DbConnectionInput) -> Result<DbConnection, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let id = Uuid::new_v4();
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO db_connections (id, environment, name, db_type, host, port, username, default_database, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id.to_string(),
                input.environment.unwrap_or_else(|| "default".to_string()),
                input.name,
                format!("{:?}", input.db_type).to_lowercase(),
                input.host,
                input.port,
                input.username,
                input.default_database,
                now.clone(),
                now,
            ],
        )
        .map_err(|e| e.to_string())?;
        drop(conn);
        self.get_db_connection(&id).map(|c| c.unwrap())
    }

    pub fn get_db_connection(&self, id: &Uuid) -> Result<Option<DbConnection>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, environment, name, db_type, host, port, username, default_database, created_at, updated_at FROM db_connections WHERE id = ?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![id.to_string()], |row| {
                Ok(DbConnection {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    environment: row.get(1)?,
                    name: row.get(2)?,
                    db_type: match row.get::<_, String>(3)?.as_str() {
                        "postgresql" => DbType::Postgresql,
                        "redis" => DbType::Redis,
                        "zookeeper" => DbType::Zookeeper,
                        "etcd" => DbType::Etcd,
                        _ => DbType::Mysql,
                    },
                    host: row.get(4)?,
                    port: row.get(5)?,
                    username: row.get(6)?,
                    default_database: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    pub fn delete_db_connection(&self, id: &Uuid) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM db_connection_secrets WHERE connection_id = ?1", params![id.to_string()])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM db_connections WHERE id = ?1", params![id.to_string()])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_db_password(&self, conn_id: &Uuid, password: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO db_connection_secrets (connection_id, password) VALUES (?1, ?2)",
            params![conn_id.to_string(), password],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_db_password(&self, conn_id: &Uuid) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT password FROM db_connection_secrets WHERE connection_id = ?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![conn_id.to_string()], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        Ok(rows.next().and_then(|r| r.ok()))
    }
}
