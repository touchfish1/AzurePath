//! Integration test for the Remote Shell session store (SQLite CRUD).
//! Run with: cargo run --example session_store_test

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use rusqlite::Connection;

    println!("{}", "=".repeat(60));
    println!("  Session Store (SQLite CRUD) Test");
    println!("{}", "=".repeat(60));

    let conn = Connection::open_in_memory()?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS remote_sessions (
            id TEXT PRIMARY KEY,
            environment TEXT NOT NULL DEFAULT 'default',
            name TEXT NOT NULL,
            protocol TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL,
            username TEXT NOT NULL DEFAULT '',
            encoding TEXT NOT NULL DEFAULT 'utf-8',
            keepalive_secs INTEGER NOT NULL DEFAULT 30,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS remote_session_secrets (
            session_id TEXT PRIMARY KEY REFERENCES remote_sessions(id) ON DELETE CASCADE,
            password TEXT NOT NULL DEFAULT '',
            private_key TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS environments (
            name TEXT PRIMARY KEY
        );
        INSERT OR IGNORE INTO environments (name) VALUES ('default');

        CREATE TABLE IF NOT EXISTS db_connections (
            id TEXT PRIMARY KEY,
            environment TEXT NOT NULL DEFAULT 'default',
            name TEXT NOT NULL,
            db_type TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL,
            username TEXT NOT NULL DEFAULT '',
            default_database TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS db_connection_secrets (
            connection_id TEXT PRIMARY KEY REFERENCES db_connections(id) ON DELETE CASCADE,
            password TEXT NOT NULL DEFAULT ''
        );
        "
    )?;
    println!("  [PASS] Schema created (5 tables)");

    // Test 1: Insert and list environments
    conn.execute("INSERT OR IGNORE INTO environments (name) VALUES ('dev'), ('staging'), ('prod')", [])?;
    let mut stmt = conn.prepare("SELECT name FROM environments ORDER BY name")?;
    let envs: Vec<String> = stmt.query_map([], |r| r.get(0))?.filter_map(|r| r.ok()).collect();
    assert_eq!(envs.len(), 4); // default + dev + staging + prod
    println!("  [PASS] Environments: {}", envs.join(", "));

    // Test 2: Create a session
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO remote_sessions (id, environment, name, protocol, host, port, username, encoding, keepalive_secs, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![id, "dev", "test-server", "Ssh", "192.168.10.220", 22, "root", "utf-8", 30, now, now],
    )?;
    println!("  [PASS] Session created: id={}", &id[..8]);

    // Test 3: Read back the session
    let mut stmt = conn.prepare(
        "SELECT id, environment, name, protocol, host, port, username, encoding, keepalive_secs FROM remote_sessions WHERE id = ?1",
    )?;
    let session = stmt.query_row(rusqlite::params![id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, u16>(5)?,
        ))
    })?;
    assert_eq!(session.0, id);
    assert_eq!(session.3, "Ssh");
    assert_eq!(session.4, "192.168.10.220");
    assert_eq!(session.5, 22u16);
    println!("  [PASS] Session read: {} @ {}:{}", session.2, session.4, session.5);

    // Test 4: Store a password
    conn.execute(
        "INSERT INTO remote_session_secrets (session_id, password) VALUES (?1, ?2)",
        rusqlite::params![id, "scmp-ppe"],
    )?;
    let pwd: String = conn.query_row(
        "SELECT password FROM remote_session_secrets WHERE session_id = ?1",
        rusqlite::params![id],
        |r| r.get(0),
    )?;
    assert_eq!(pwd, "scmp-ppe");
    println!("  [PASS] Password stored and retrieved");

    // Test 5: Update session
    let new_time = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE remote_sessions SET name=?1, host=?2, updated_at=?3 WHERE id=?4",
        rusqlite::params!["test-server-updated", "192.168.10.221", new_time, id],
    )?;
    let updated_name: String = conn.query_row(
        "SELECT name FROM remote_sessions WHERE id = ?1",
        rusqlite::params![id],
        |r| r.get(0),
    )?;
    assert_eq!(updated_name, "test-server-updated");
    println!("  [PASS] Session updated: {}", updated_name);

    // Test 6: List all sessions
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM remote_sessions", [], |r| r.get(0))?;
    assert_eq!(count, 1);
    println!("  [PASS] Session count: {}", count);

    // Test 7: Delete session (cascade should delete secret too)
    conn.execute("DELETE FROM remote_sessions WHERE id = ?1", rusqlite::params![id])?;
    let remaining: i64 = conn.query_row("SELECT COUNT(*) FROM remote_sessions", [], |r| r.get(0))?;
    assert_eq!(remaining, 0);
    let secret_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM remote_session_secrets WHERE session_id = ?1",
        rusqlite::params![id],
        |r| r.get(0),
    )?;
    assert_eq!(secret_count, 0);
    println!("  [PASS] Session deleted (cascade verified)");

    // Test 8: DB Connection CRUD
    let db_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO db_connections (id, environment, name, db_type, host, port, username, default_database, created_at, updated_at)
         VALUES (?1, 'dev', 'test-mysql', 'mysql', '192.168.10.220', 3306, 'root', 'testdb', ?2, ?2)",
        rusqlite::params![db_id, chrono::Utc::now().to_rfc3339()],
    )?;
    conn.execute(
        "INSERT INTO db_connection_secrets (connection_id, password) VALUES (?1, ?2)",
        rusqlite::params![db_id, "mysql_pass"],
    )?;
    let db_count: i64 = conn.query_row("SELECT COUNT(*) FROM db_connections", [], |r| r.get(0))?;
    assert_eq!(db_count, 1);
    println!("  [PASS] DB connection created");

    // Test 9: Environment isolation
    let prod_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO remote_sessions (id, environment, name, protocol, host, port, username, encoding, keepalive_secs, created_at, updated_at)
         VALUES (?1, 'prod', 'prod-server', 'Ssh', '10.0.0.1', 22, 'admin', 'utf-8', 30, ?2, ?2)",
        rusqlite::params![prod_id, chrono::Utc::now().to_rfc3339()],
    )?;
    let dev_sessions: i64 = conn.query_row(
        "SELECT COUNT(*) FROM remote_sessions WHERE environment = 'dev'",
        [],
        |r| r.get(0),
    )?;
    let prod_sessions: i64 = conn.query_row(
        "SELECT COUNT(*) FROM remote_sessions WHERE environment = 'prod'",
        [],
        |r| r.get(0),
    )?;
    assert_eq!(dev_sessions, 0); // dev session was deleted
    assert_eq!(prod_sessions, 1);
    println!("  [PASS] Environment isolation: dev=0, prod=1");

    // Test 10: UTF-8 encoding handling (Chinese characters in session name)
    let cn_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO remote_sessions (id, environment, name, protocol, host, port, username, encoding, keepalive_secs, created_at, updated_at)
         VALUES (?1, 'default', '生产服务器', 'Ssh', '192.168.10.220', 22, 'root', 'utf-8', 30, ?2, ?2)",
        rusqlite::params![cn_id, chrono::Utc::now().to_rfc3339()],
    )?;
    let cn_name: String = conn.query_row(
        "SELECT name FROM remote_sessions WHERE id = ?1",
        rusqlite::params![cn_id],
        |r| r.get(0),
    )?;
    assert_eq!(cn_name, "生产服务器");
    println!("  [PASS] UTF-8 Chinese session name: {}", cn_name);

    // Cleanup
    conn.execute("DELETE FROM remote_sessions WHERE id = ?1", rusqlite::params![cn_id])?;
    conn.execute("DELETE FROM db_connections WHERE id = ?1", rusqlite::params![db_id])?;
    println!("  [PASS] Cleanup completed");

    println!("\n{}", "=".repeat(60));
    println!("  ALL 10 SQLITE CRUD TESTS PASSED");
    println!("{}", "=".repeat(60));

    Ok(())
}
