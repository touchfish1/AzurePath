//! Test MySQL connection using sqlx (same as the remote shell backend).
//! Run with: cargo run --example mysql_test

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "=".repeat(60));
    println!("  MySQL Integration Test (via sqlx)");
    println!("{}", "=".repeat(60));

    let host = "192.168.10.220";
    let mysql_pass = std::env::var("MYSQL_PASS").unwrap_or_else(|_| "scmp-ppe".to_string());
    let url = format!("mysql://root:{mysql_pass}@{host}:3306");

    println!("\n[Test 1] TCP Connection to {host}:3306");
    match std::net::TcpStream::connect_timeout(
        &format!("{host}:3306").parse()?,
        std::time::Duration::from_secs(5),
    ) {
        Ok(_) => println!("  [PASS] MySQL port 3306 reachable"),
        Err(e) => {
            println!("  [SKIP] MySQL not reachable: {e}");
            return Ok(());
        }
    }

    println!("\n[Test 2] Connecting with sqlx...");
    let pool = match sqlx::mysql::MySqlPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&url)
        .await
    {
        Ok(p) => {
            println!("  [PASS] MySQL connection established");
            p
        }
        Err(e) => {
            println!("  [SKIP] MySQL connection failed: {e}");
            return Ok(());
        }
    };

    println!("\n[Test 3] List Databases");
    let rows: Vec<(String,)> = sqlx::query_as("SELECT SCHEMA_NAME FROM information_schema.SCHEMATA")
        .fetch_all(&pool)
        .await?;
    println!("  [PASS] Found {} databases", rows.len());
    for (name,) in rows.iter().take(10) {
        println!("    - {name}");
    }

    println!("\n[Test 4] Execute SELECT 1");
    let result: (i32,) = sqlx::query_as("SELECT 1 AS test_col")
        .fetch_one(&pool)
        .await?;
    assert_eq!(result.0, 1);
    println!("  [PASS] SELECT 1 = {}", result.0);

    println!("\n[Test 5] Version check");
    let row: (String,) = sqlx::query_as("SELECT VERSION()")
        .fetch_one(&pool)
        .await?;
    println!("  [PASS] MySQL version: {}", row.0);

    println!("\n[Test 6] List Tables in first non-system database");
    let db_names: Vec<(String,)> = sqlx::query_as(
        "SELECT SCHEMA_NAME FROM information_schema.SCHEMATA
         WHERE SCHEMA_NAME NOT IN ('information_schema', 'mysql', 'performance_schema', 'sys')
         LIMIT 1",
    )
    .fetch_all(&pool)
    .await?;

    if let Some((db_name,)) = db_names.first() {
        let query = format!(
            "SELECT TABLE_NAME FROM information_schema.TABLES WHERE TABLE_SCHEMA = '{}' LIMIT 10",
            db_name.replace('\'', "''")
        );
        let tables: Vec<(String,)> = sqlx::query_as(&query).fetch_all(&pool).await?;
        println!("  [PASS] Database '{db_name}' has {} tables", tables.len());
        for (t,) in tables.iter() {
            println!("    - {t}");
        }
    } else {
        println!("  [SKIP] No user databases found");
    }

    println!("\n[Test 7] SQL EXPLAIN test");
    let explain_rows: Vec<sqlx::mysql::MySqlRow> = sqlx::query("EXPLAIN SELECT 1")
        .fetch_all(&pool)
        .await?;
    println!("  [PASS] EXPLAIN returned {} row(s)", explain_rows.len());

    pool.close().await;
    println!("\n{}", "=".repeat(60));
    println!("  ALL MYSQL TESTS PASSED");
    println!("{}", "=".repeat(60));

    Ok(())
}
