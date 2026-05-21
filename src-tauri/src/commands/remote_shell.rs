//! Remote shell Tauri commands — session management, terminal I/O, SFTP, monitoring, databases.

use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::sync::Arc;
use std::sync::OnceLock;
use sqlx::Column;
use tokio::sync::Mutex;

use crate::core::remote_shell::session_store::SessionStore;
use crate::core::remote_shell::ssh::SshClient;
use crate::core::remote_shell::telnet::TelnetClient;
use crate::types::remote_shell::database::{
    DbConnection, DbConnectionInput, DbType, MySqlColumnInfo, MySqlQueryResult, RedisKeyEntry,
};
use crate::types::remote_shell::host_metrics::HostMetrics;
use crate::types::remote_shell::session::{Protocol, RemoteSession, SessionInput, SessionSummary};
use crate::types::remote_shell::sftp::{SftpEntry, SftpTextReadResult};
use crate::types::remote_shell::terminal::{TerminalClient, TerminalError};

static STORE: OnceLock<Arc<SessionStore>> = OnceLock::new();
static ACTIVE_TERMINALS: OnceLock<Arc<Mutex<HashMap<String, Box<dyn TerminalClient + Send>>>>> =
    OnceLock::new();
static SSH_PASSWORDS: OnceLock<Arc<Mutex<HashMap<String, String>>>> = OnceLock::new();
static DB_PASSWORDS: OnceLock<Arc<Mutex<HashMap<String, String>>>> = OnceLock::new();

fn store() -> &'static Arc<SessionStore> {
    STORE.get().expect("SessionStore not initialized")
}

fn active() -> &'static Arc<Mutex<HashMap<String, Box<dyn TerminalClient + Send>>>> {
    ACTIVE_TERMINALS.get().expect("ACTIVE_TERMINALS not initialized")
}

fn ssh_passwords() -> &'static Arc<Mutex<HashMap<String, String>>> {
    SSH_PASSWORDS.get().expect("SSH_PASSWORDS not initialized")
}

fn db_passwords() -> &'static Arc<Mutex<HashMap<String, String>>> {
    DB_PASSWORDS.get().expect("DB_PASSWORDS not initialized")
}

#[tauri::command]
pub async fn remote_shell_init() -> Result<(), String> {
    let store = SessionStore::new().map_err(|e| format!("SessionStore init failed: {e}"))?;
    STORE.get_or_init(|| Arc::new(store));
    ACTIVE_TERMINALS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
    SSH_PASSWORDS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
    DB_PASSWORDS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
    Ok(())
}

// ── Session CRUD ──

#[tauri::command]
pub async fn remote_shell_list_sessions() -> Result<Vec<RemoteSession>, String> {
    store().list_sessions()
}

#[tauri::command]
pub async fn remote_shell_get_session(id: String) -> Result<RemoteSession, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    store().get_session(&uuid)?.ok_or_else(|| "Session not found".to_string())
}

#[tauri::command]
pub async fn remote_shell_create_session(input: SessionInput, password: String) -> Result<RemoteSession, String> {
    let session = store().create_session(input)?;
    store().set_session_password(&session.id, &password)?;
    ssh_passwords()
        .lock()
        .await
        .insert(session.id.to_string(), password);
    Ok(session)
}

#[tauri::command]
pub async fn remote_shell_update_session(id: String, input: SessionInput) -> Result<RemoteSession, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    store().update_session(&uuid, input)
}

#[tauri::command]
pub async fn remote_shell_delete_session(id: String) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    active().lock().await.remove(&id);
    store().delete_session(&uuid)
}

// ── Terminal Connection ──

#[tauri::command]
pub async fn remote_shell_connect(id: String) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let session = store().get_session(&uuid)?.ok_or_else(|| "Session not found".to_string())?;

    let password = store().get_session_password(&uuid)?.unwrap_or_default();

    let mut client: Box<dyn TerminalClient + Send> = match session.protocol {
        Protocol::Ssh => Box::new(SshClient::new(Some(password.clone()))),
        Protocol::Telnet => Box::new(TelnetClient::new()),
    };

    client.connect(&session, &password).await.map_err(|e| e.to_string())?;
    active().lock().await.insert(id, client);
    Ok(())
}

#[tauri::command]
pub async fn remote_shell_disconnect(id: String) -> Result<(), String> {
    let mut map = active().lock().await;
    if let Some(mut client) = map.remove(&id) {
        client.disconnect().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn remote_shell_send_input(id: String, data: String) -> Result<(), String> {
    let mut map = active().lock().await;
    let client = map.get_mut(&id).ok_or_else(|| TerminalError::SessionNotFound.to_string())?;
    client.write(data.as_bytes()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remote_shell_pull_output(id: String) -> Result<String, String> {
    let mut map = active().lock().await;
    let client = map.get_mut(&id).ok_or_else(|| TerminalError::SessionNotFound.to_string())?;
    let data = client.read().await.map_err(|e| e.to_string())?;
    Ok(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data))
}

#[tauri::command]
pub async fn remote_shell_resize(id: String, cols: u16, rows: u16) -> Result<(), String> {
    let mut map = active().lock().await;
    if let Some(client) = map.get_mut(&id) {
        client.resize(cols, rows).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn remote_shell_list_summaries() -> Result<Vec<SessionSummary>, String> {
    let sessions = store().list_sessions()?;
    let active_map = active().lock().await;
    Ok(sessions
        .into_iter()
        .map(|s| SessionSummary {
            id: s.id,
            environment: s.environment,
            name: s.name,
            protocol: s.protocol,
            host: s.host,
            port: s.port,
            username: s.username,
            is_connected: active_map.contains_key(&s.id.to_string()),
        })
        .collect())
}

// ── SFTP ──

fn ssh_connect(host: &str, port: u16, username: &str, password: &str) -> Result<ssh2::Session, String> {
    let tcp = std::net::TcpStream::connect(format!("{}:{}", host, port)).map_err(|e| e.to_string())?;
    tcp.set_read_timeout(Some(std::time::Duration::from_secs(10))).ok();
    let mut sess = ssh2::Session::new().map_err(|e| e.to_string())?;
    sess.set_tcp_stream(tcp);
    sess.handshake().map_err(|e| e.to_string())?;
    sess.userauth_password(username, password).map_err(|e| e.to_string())?;
    Ok(sess)
}

#[tauri::command]
pub async fn remote_shell_list_sftp(session_id: String, path: String) -> Result<Vec<SftpEntry>, String> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    let password = store()
        .get_session_password(&uuid)?
        .ok_or_else(|| "No password stored".to_string())?;
    let session = store()
        .get_session(&uuid)?
        .ok_or_else(|| "Session not found".to_string())?;

    let sess = ssh_connect(&session.host, session.port, &session.username, &password)?;
    let sftp = sess.sftp().map_err(|e| e.to_string())?;

    let entries = sftp
        .readdir(std::path::Path::new(&path))
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|(p, _): &(std::path::PathBuf, ssh2::FileStat)| {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            name != "." && name != ".."
        })
        .map(|(p, stat): (std::path::PathBuf, ssh2::FileStat)| SftpEntry {
            name: p.file_name().unwrap_or_default().to_string_lossy().to_string(),
            path: p.to_string_lossy().to_string(),
            is_dir: stat.is_dir(),
            size: stat.size.unwrap_or(0),
            mtime: stat.mtime.unwrap_or(0),
        })
        .collect();
    Ok(entries)
}

#[tauri::command]
pub async fn remote_shell_read_sftp_text(session_id: String, path: String) -> Result<SftpTextReadResult, String> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    let password = store()
        .get_session_password(&uuid)?
        .ok_or_else(|| "No password stored".to_string())?;
    let session = store()
        .get_session(&uuid)?
        .ok_or_else(|| "Session not found".to_string())?;

    let sess = ssh_connect(&session.host, session.port, &session.username, &password)?;
    let sftp = sess.sftp().map_err(|e| e.to_string())?;
    let mut file = sftp.open(std::path::Path::new(&path)).map_err(|e| e.to_string())?;
    let mut content = String::new();
    file.read_to_string(&mut content).map_err(|e| e.to_string())?;
    Ok(SftpTextReadResult {
        content,
        encoding: "utf-8".to_string(),
    })
}

#[tauri::command]
pub async fn remote_shell_save_sftp_text(session_id: String, path: String, content: String) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    let password = store()
        .get_session_password(&uuid)?
        .ok_or_else(|| "No password stored".to_string())?;
    let session = store()
        .get_session(&uuid)?
        .ok_or_else(|| "Session not found".to_string())?;

    let sess = ssh_connect(&session.host, session.port, &session.username, &password)?;
    let sftp = sess.sftp().map_err(|e| e.to_string())?;
    let mut file = sftp.create(std::path::Path::new(&path)).map_err(|e| e.to_string())?;
    file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Host Metrics ──

#[tauri::command]
pub async fn remote_shell_get_metrics(session_id: String) -> Result<HostMetrics, String> {
    let uuid = uuid::Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    let password = store()
        .get_session_password(&uuid)?
        .ok_or_else(|| "No password stored".to_string())?;
    let session = store()
        .get_session(&uuid)?
        .ok_or_else(|| "Session not found".to_string())?;

    let sess = ssh_connect(&session.host, session.port, &session.username, &password)?;

    let cpu = run_ssh_command(&sess, "top -bn1 | grep 'Cpu(s)' | awk '{print $2}' | cut -d'%' -f1")
        .unwrap_or_else(|_| "0".to_string());
    let mem_info = run_ssh_command(&sess, "free -b | grep Mem: | awk '{print $2,$3}'").unwrap_or_default();
    let disk_info = run_ssh_command(&sess, "df -B1 / | tail -1 | awk '{print $2,$3}'").unwrap_or_default();

    let cpu_percent: f64 = cpu.trim().parse().unwrap_or(0.0);

    let mem_parts: Vec<&str> = mem_info.trim().split_whitespace().collect();
    let mem_total: u64 = mem_parts.first().and_then(|s| s.parse().ok()).unwrap_or(1);
    let mem_used: u64 = mem_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    let disk_parts: Vec<&str> = disk_info.trim().split_whitespace().collect();
    let disk_total: u64 = disk_parts.first().and_then(|s| s.parse().ok()).unwrap_or(1);
    let disk_used: u64 = disk_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    Ok(HostMetrics {
        cpu_percent,
        memory_used_bytes: mem_used,
        memory_total_bytes: mem_total,
        memory_percent: if mem_total > 0 {
            (mem_used as f64 / mem_total as f64) * 100.0
        } else {
            0.0
        },
        disk_used_bytes: disk_used,
        disk_total_bytes: disk_total,
        disk_percent: if disk_total > 0 {
            (disk_used as f64 / disk_total as f64) * 100.0
        } else {
            0.0
        },
        collected_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn run_ssh_command(sess: &ssh2::Session, cmd: &str) -> Result<String, String> {
    let mut channel = sess.channel_session().map_err(|e| e.to_string())?;
    channel.exec(cmd).map_err(|e| e.to_string())?;
    let mut output = String::new();
    channel.read_to_string(&mut output).map_err(|e| e.to_string())?;
    channel.close().map_err(|e| e.to_string())?;
    channel.wait_close().map_err(|e| e.to_string())?;
    Ok(output)
}

// ── Environment ──

#[tauri::command]
pub async fn remote_shell_list_environments() -> Result<Vec<String>, String> {
    store().list_environments()
}

#[tauri::command]
pub async fn remote_shell_create_environment(name: String) -> Result<(), String> {
    store().create_environment(&name)
}

// ── Database Connections ──

#[tauri::command]
pub async fn remote_shell_list_db_connections(db_type: Option<String>) -> Result<Vec<DbConnection>, String> {
    store().list_db_connections(db_type.as_deref())
}

#[tauri::command]
pub async fn remote_shell_create_db_connection(input: DbConnectionInput, password: String) -> Result<DbConnection, String> {
    let conn = store().create_db_connection(input)?;
    store().set_db_password(&conn.id, &password)?;
    db_passwords()
        .lock()
        .await
        .insert(conn.id.to_string(), password);
    Ok(conn)
}

#[tauri::command]
pub async fn remote_shell_delete_db_connection(id: String) -> Result<(), String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    store().delete_db_connection(&uuid)
}

#[tauri::command]
pub async fn remote_shell_test_db_connection(id: String) -> Result<String, String> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let conn = store().get_db_connection(&uuid)?.ok_or_else(|| "Connection not found".to_string())?;
    let password = store().get_db_password(&uuid)?.unwrap_or_default();

    match conn.db_type {
        DbType::Mysql => {
            use sqlx::mysql::MySqlPoolOptions;
            let url = format!("mysql://{}:{}@{}:{}/{}", conn.username, password, conn.host, conn.port, conn.default_database.as_deref().unwrap_or("mysql"));
            let pool: sqlx::MySqlPool = MySqlPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(std::time::Duration::from_secs(5))
                .connect(&url)
                .await
                .map_err(|e: sqlx::Error| e.to_string())?;
            pool.close().await;
            Ok("ok".to_string())
        }
        DbType::Postgresql => {
            use sqlx::postgres::PgPoolOptions;
            let url = format!("postgresql://{}:{}@{}:{}/{}", conn.username, password, conn.host, conn.port, conn.default_database.as_deref().unwrap_or("postgres"));
            let pool: sqlx::PgPool = PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(std::time::Duration::from_secs(5))
                .connect(&url)
                .await
                .map_err(|e: sqlx::Error| e.to_string())?;
            pool.close().await;
            Ok("ok".to_string())
        }
        DbType::Redis => {
            let url = format!("redis://:{}@{}:{}/0", password, conn.host, conn.port);
            let client = redis::Client::open(url.as_str()).map_err(|e| e.to_string())?;
            let mut conn2 = client.get_multiplexed_tokio_connection().await.map_err(|e| e.to_string())?;
            let _: String = redis::cmd("PING").query_async(&mut conn2).await.map_err(|e| e.to_string())?;
            Ok("ok".to_string())
        }
        DbType::Zookeeper => {
            // Basic TCP connectivity check
            tokio::net::TcpStream::connect(format!("{}:{}", conn.host, conn.port))
                .await
                .map_err(|e| e.to_string())?;
            Ok("ok".to_string())
        }
        DbType::Etcd => {
            // Basic TCP connectivity check
            tokio::net::TcpStream::connect(format!("{}:{}", conn.host, conn.port))
                .await
                .map_err(|e| e.to_string())?;
            Ok("ok".to_string())
        }
    }
}

// ── MySQL ──

#[tauri::command]
pub async fn remote_shell_mysql_list_databases(conn_id: String) -> Result<Vec<String>, String> {
    let (url, _) = get_mysql_url(&conn_id).await?;
    use sqlx::mysql::MySqlPoolOptions;
    let pool = MySqlPoolOptions::new().max_connections(1).connect(&url).await.map_err(|e| e.to_string())?;
    let rows: Vec<(String,)> = sqlx::query_as("SHOW DATABASES").fetch_all(&pool).await.map_err(|e| e.to_string())?;
    pool.close().await;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

#[tauri::command]
pub async fn remote_shell_mysql_list_tables(conn_id: String, database: String) -> Result<Vec<String>, String> {
    let (url, _) = get_mysql_url(&conn_id).await?;
    let url = url.replace("/mysql", &format!("/{}", database));
    use sqlx::mysql::MySqlPoolOptions;
    let pool = MySqlPoolOptions::new().max_connections(1).connect(&url).await.map_err(|e| e.to_string())?;
    let rows: Vec<(String,)> = sqlx::query_as("SHOW TABLES").fetch_all(&pool).await.map_err(|e| e.to_string())?;
    pool.close().await;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

#[tauri::command]
pub async fn remote_shell_mysql_describe_table(conn_id: String, database: String, table: String) -> Result<Vec<MySqlColumnInfo>, String> {
    let (url, _) = get_mysql_url(&conn_id).await?;
    let url = url.replace("/mysql", &format!("/{}", database));
    use sqlx::mysql::MySqlPoolOptions;
    let pool = MySqlPoolOptions::new().max_connections(1).connect(&url).await.map_err(|e| e.to_string())?;
    let rows: Vec<MySqlColumnInfo> = sqlx::query_as::<_, (String, String, String, String, Option<String>, String)>(
        &format!("DESCRIBE `{}`", table),
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?
    .into_iter()
    .map(|(f, t, null, key, dflt, extra)| MySqlColumnInfo {
        field: f,
        db_type: t,
        nullable: null == "YES",
        key,
        default: dflt,
        extra,
    })
    .collect();
    pool.close().await;
    Ok(rows)
}

#[tauri::command]
pub async fn remote_shell_mysql_execute_query(conn_id: String, database: String, query: String) -> Result<MySqlQueryResult, String> {
    let (url, _) = get_mysql_url(&conn_id).await?;
    let url = url.replace("/mysql", &format!("/{}", database));
    use sqlx::mysql::MySqlPoolOptions;
    let pool = MySqlPoolOptions::new().max_connections(1).connect(&url).await.map_err(|e| e.to_string())?;
    let start = std::time::Instant::now();

    let trimmed = query.trim().to_uppercase();
    if trimmed.starts_with("SELECT") || trimmed.starts_with("SHOW") || trimmed.starts_with("DESCRIBE") || trimmed.starts_with("EXPLAIN") {
        use sqlx::Row;
        let rows = sqlx::query(&query).fetch_all(&pool).await.map_err(|e| e.to_string())?;
        let elapsed = start.elapsed().as_millis() as u64;
        let columns: Vec<String> = if !rows.is_empty() {
            rows[0].columns().iter().map(|c| c.name().to_string()).collect()
        } else {
            vec![]
        };
        let values: Vec<Vec<serde_json::Value>> = rows
            .into_iter()
            .map(|row| {
                (0..columns.len())
                    .map(|i| {
                        let val: Result<String, _> = row.try_get(i);
                        match val {
                            Ok(s) => serde_json::Value::String(s),
                            Err(_) => serde_json::Value::Null,
                        }
                    })
                    .collect()
            })
            .collect();
        pool.close().await;
        Ok(MySqlQueryResult {
            columns,
            rows: values,
            affected_rows: 0,
            elapsed_ms: elapsed,
        })
    } else {
        let affected = sqlx::query(&query).execute(&pool).await.map_err(|e| e.to_string())?;
        let elapsed = start.elapsed().as_millis() as u64;
        pool.close().await;
        Ok(MySqlQueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows: affected.rows_affected(),
            elapsed_ms: elapsed,
        })
    }
}

async fn get_mysql_url(conn_id: &str) -> Result<(String, String), String> {
    let uuid = uuid::Uuid::parse_str(conn_id).map_err(|e| e.to_string())?;
    let conn = store()
        .get_db_connection(&uuid)?
        .ok_or_else(|| "Connection not found".to_string())?;
    let password = store()
        .get_db_password(&uuid)?
        .unwrap_or_default();
    let url = format!(
        "mysql://{}:{}@{}:{}/mysql",
        conn.username, password, conn.host, conn.port
    );
    Ok((url, password))
}

// ── PostgreSQL ──

#[tauri::command]
pub async fn remote_shell_pg_list_databases(conn_id: String) -> Result<Vec<String>, String> {
    let (url, _) = get_pg_url(&conn_id).await?;
    use sqlx::postgres::PgPoolOptions;
    let pool = PgPoolOptions::new().max_connections(1).connect(&url).await.map_err(|e| e.to_string())?;
    let rows: Vec<(String,)> = sqlx::query_as("SELECT datname FROM pg_database WHERE datistemplate = false")
        .fetch_all(&pool)
        .await
        .map_err(|e| e.to_string())?;
    pool.close().await;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

#[tauri::command]
pub async fn remote_shell_pg_list_tables(conn_id: String, database: String) -> Result<Vec<String>, String> {
    let (url, _) = get_pg_url(&conn_id).await?;
    let url = url.replace("/postgres", &format!("/{}", database));
    use sqlx::postgres::PgPoolOptions;
    let pool = PgPoolOptions::new().max_connections(1).connect(&url).await.map_err(|e| e.to_string())?;
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname NOT IN ('pg_catalog', 'information_schema')",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| e.to_string())?;
    pool.close().await;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

#[tauri::command]
pub async fn remote_shell_pg_execute_query(conn_id: String, database: String, query: String) -> Result<MySqlQueryResult, String> {
    let (url, _) = get_pg_url(&conn_id).await?;
    let url = url.replace("/postgres", &format!("/{}", database));
    use sqlx::postgres::PgPoolOptions;
    let pool = PgPoolOptions::new().max_connections(1).connect(&url).await.map_err(|e| e.to_string())?;
    let start = std::time::Instant::now();

    let trimmed = query.trim().to_uppercase();
    if trimmed.starts_with("SELECT") || trimmed.starts_with("SHOW") || trimmed.starts_with("WITH") {
        use sqlx::Row;
        let rows = sqlx::query(&query).fetch_all(&pool).await.map_err(|e| e.to_string())?;
        let elapsed = start.elapsed().as_millis() as u64;
        let columns: Vec<String> = if !rows.is_empty() {
            rows[0].columns().iter().map(|c| c.name().to_string()).collect()
        } else {
            vec![]
        };
        let values: Vec<Vec<serde_json::Value>> = rows
            .into_iter()
            .map(|row| {
                (0..columns.len())
                    .map(|i| {
                        let val: Result<String, _> = row.try_get(i);
                        match val {
                            Ok(s) => serde_json::Value::String(s),
                            Err(_) => serde_json::Value::Null,
                        }
                    })
                    .collect()
            })
            .collect();
        pool.close().await;
        Ok(MySqlQueryResult {
            columns,
            rows: values,
            affected_rows: 0,
            elapsed_ms: elapsed,
        })
    } else {
        let affected = sqlx::query(&query).execute(&pool).await.map_err(|e| e.to_string())?;
        let elapsed = start.elapsed().as_millis() as u64;
        pool.close().await;
        Ok(MySqlQueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows: affected.rows_affected(),
            elapsed_ms: elapsed,
        })
    }
}

async fn get_pg_url(conn_id: &str) -> Result<(String, String), String> {
    let uuid = uuid::Uuid::parse_str(conn_id).map_err(|e| e.to_string())?;
    let conn = store()
        .get_db_connection(&uuid)?
        .ok_or_else(|| "Connection not found".to_string())?;
    let password = store()
        .get_db_password(&uuid)?
        .unwrap_or_default();
    let url = format!(
        "postgresql://{}:{}@{}:{}/postgres",
        conn.username, password, conn.host, conn.port
    );
    Ok((url, password))
}

// ── Redis ──

#[tauri::command]
pub async fn remote_shell_redis_list_keys(conn_id: String, pattern: Option<String>) -> Result<Vec<RedisKeyEntry>, String> {
    let (url, _) = get_redis_url(&conn_id).await?;
    let client = redis::Client::open(url.as_str()).map_err(|e| e.to_string())?;
    let mut conn = client.get_multiplexed_tokio_connection().await.map_err(|e| e.to_string())?;
    let pat = pattern.unwrap_or_else(|| "*".to_string());
    let keys: Vec<String> = redis::cmd("KEYS").arg(&pat).query_async(&mut conn).await.map_err(|e| e.to_string())?;

    let mut entries = Vec::new();
    for key in keys {
        let key_type: String = redis::cmd("TYPE").arg(&key).query_async(&mut conn).await.unwrap_or_else(|_| "none".to_string());
        let ttl: i64 = redis::cmd("TTL").arg(&key).query_async(&mut conn).await.unwrap_or(-2);
        entries.push(RedisKeyEntry {
            key,
            key_type,
            ttl,
            size: 0,
        });
    }
    Ok(entries)
}

#[tauri::command]
pub async fn remote_shell_redis_get_value(conn_id: String, key: String) -> Result<String, String> {
    let (url, _) = get_redis_url(&conn_id).await?;
    let client = redis::Client::open(url.as_str()).map_err(|e| e.to_string())?;
    let mut conn = client.get_multiplexed_tokio_connection().await.map_err(|e| e.to_string())?;
    let val: Option<String> = redis::cmd("GET").arg(&key).query_async(&mut conn).await.map_err(|e| e.to_string())?;
    Ok(val.unwrap_or_default())
}

#[tauri::command]
pub async fn remote_shell_redis_set_value(conn_id: String, key: String, value: String) -> Result<(), String> {
    let (url, _) = get_redis_url(&conn_id).await?;
    let client = redis::Client::open(url.as_str()).map_err(|e| e.to_string())?;
    let mut conn = client.get_multiplexed_tokio_connection().await.map_err(|e| e.to_string())?;
    redis::cmd("SET").arg(&key).arg(&value).query_async::<()>(&mut conn).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remote_shell_redis_set_ttl(conn_id: String, key: String, ttl: i64) -> Result<(), String> {
    let (url, _) = get_redis_url(&conn_id).await?;
    let client = redis::Client::open(url.as_str()).map_err(|e| e.to_string())?;
    let mut conn = client.get_multiplexed_tokio_connection().await.map_err(|e| e.to_string())?;
    if ttl > 0 {
        redis::cmd("EXPIRE").arg(&key).arg(ttl).query_async::<()>(&mut conn).await.map_err(|e| e.to_string())
    } else {
        redis::cmd("PERSIST").arg(&key).query_async::<()>(&mut conn).await.map_err(|e| e.to_string())
    }
}

async fn get_redis_url(conn_id: &str) -> Result<(String, String), String> {
    let uuid = uuid::Uuid::parse_str(conn_id).map_err(|e| e.to_string())?;
    let conn = store()
        .get_db_connection(&uuid)?
        .ok_or_else(|| "Connection not found".to_string())?;
    let password = store()
        .get_db_password(&uuid)?
        .unwrap_or_default();
    let url = format!("redis://:{}@{}:{}/0", password, conn.host, conn.port);
    Ok((url, password))
}
