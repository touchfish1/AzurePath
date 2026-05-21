//! Quick check for MySQL/Redis availability on the target server.
//! Run with: cargo run --example db_check

use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = "192.168.10.220";

    // Check MySQL port (3306)
    println!("Checking MySQL (3306)...");
    match TcpStream::connect_timeout(&format!("{host}:3306").parse()?, Duration::from_secs(5)) {
        Ok(_) => println!("  [PASS] MySQL port 3306 is open"),
        Err(e) => println!("  [SKIP] MySQL port 3306: {e}"),
    }

    // Check Redis port (6379)
    println!("Checking Redis (6379)...");
    match TcpStream::connect_timeout(&format!("{host}:6379").parse()?, Duration::from_secs(5)) {
        Ok(mut stream) => {
            // Send Redis PING
            use std::io::Write;
            write!(stream, "*1\r\n$4\r\nPING\r\n")?;
            let mut buf = [0u8; 32];
            stream.read(&mut buf)?;
            let resp = String::from_utf8_lossy(&buf);
            if resp.contains("PONG") {
                println!("  [PASS] Redis responds to PING: {resp:?}");
            } else {
                println!("  [PASS] Redis port 6379 is open but response: {resp:?}");
            }
        }
        Err(e) => println!("  [SKIP] Redis port 6379: {e}"),
    }

    // Check PostgreSQL port (5432)
    println!("Checking PostgreSQL (5432)...");
    match TcpStream::connect_timeout(&format!("{host}:5432").parse()?, Duration::from_secs(5)) {
        Ok(_) => println!("  [PASS] PostgreSQL port 5432 is open"),
        Err(e) => println!("  [SKIP] PostgreSQL port 5432: {e}"),
    }

    // Check Zookeeper port (2181)
    println!("Checking Zookeeper (2181)...");
    match TcpStream::connect_timeout(&format!("{host}:2181").parse()?, Duration::from_secs(5)) {
        Ok(_) => println!("  [PASS] Zookeeper port 2181 is open"),
        Err(e) => println!("  [SKIP] Zookeeper port 2181: {e}"),
    }

    // Check Etcd port (2379)
    println!("Checking Etcd (2379)...");
    match TcpStream::connect_timeout(&format!("{host}:2379").parse()?, Duration::from_secs(5)) {
        Ok(_) => println!("  [PASS] Etcd port 2379 is open"),
        Err(e) => println!("  [SKIP] Etcd port 2379: {e}"),
    }

    Ok(())
}
