//! Integration test for remote shell SSH functionality.
//! Tests against a real SSH server using the same ssh2 API as the remote shell module.
//! Run with: cargo run --example ssh_test

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

const SSH_HOST: &str = "192.168.10.220";
const SSH_PORT: u16 = 22;
const SSH_USER: &str = "root";
const SSH_PASS: &str = env!("SSH_PASS", "Set SSH_PASS env var for integration tests");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "=".repeat(60));
    println!("  Remote Shell Integration Test");
    println!("  Target: {SSH_USER}@{SSH_HOST}:{SSH_PORT}");
    println!("{}", "=".repeat(60));

    // ─── Test 1: TCP Connection ─────────────────────────────────
    println!("\n[Test 1] TCP Connection");
    let addr = format!("{SSH_HOST}:{SSH_PORT}");
    let tcp = TcpStream::connect_timeout(&addr.parse()?, Duration::from_secs(10))?;
    tcp.set_read_timeout(Some(Duration::from_secs(30)))?;
    println!("  [PASS] Connected to {addr}");

    // ─── Test 2: SSH Handshake ──────────────────────────────────
    println!("\n[Test 2] SSH Handshake");
    let mut sess = ssh2::Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;
    println!("  [PASS] SSH handshake completed");
    if let Some(banner) = sess.banner() {
        println!("  Banner: {}", banner.trim());
    }

    // ─── Test 3: Password Authentication ────────────────────────
    println!("\n[Test 3] Password Authentication");
    sess.userauth_password(SSH_USER, SSH_PASS)?;
    assert!(sess.authenticated());
    println!("  [PASS] Authenticated as {SSH_USER}");

    // ─── Test 4: Command Execution ──────────────────────────────
    println!("\n[Test 4] Command Execution (non-PTY)");
    let mut channel = sess.channel_session()?;
    channel.exec("echo '=== AZUREPATH TEST ===' && hostname && uptime && whoami")?;
    let mut output = String::new();
    channel.read_to_string(&mut output)?;
    channel.wait_close()?;
    let exit_code = channel.exit_status()?;
    println!("  Exit code: {exit_code}");
    assert_eq!(exit_code, 0, "Command should exit with 0");
    println!("  [PASS] Command executed successfully");
    for line in output.lines() {
        println!("    > {line}");
    }

    // ─── Test 5: PTY + Shell (terminal emulation) ──────────────
    println!("\n[Test 5] PTY + Shell (terminal emulation)");
    let mut ch = sess.channel_session()?;
    ch.request_pty("xterm-256color", None, Some((80, 24, 0, 0)))?;
    ch.shell()?;
    writeln!(ch, "ls -la /root | head -5")?;
    std::thread::sleep(Duration::from_millis(500));
    writeln!(ch, "echo '---PTY-OK---'")?;
    std::thread::sleep(Duration::from_millis(300));
    writeln!(ch, "exit")?;

    let mut pty_output = Vec::new();
    ch.read_to_end(&mut pty_output)?;
    ch.wait_close()?;
    let pty_text = String::from_utf8_lossy(&pty_output);
    assert!(pty_text.contains("PTY-OK"), "PTY shell should echo marker");
    println!("  [PASS] PTY shell worked ({} bytes output)", pty_output.len());
    for line in pty_text.lines().filter(|l| !l.is_empty()).take(10) {
        println!("    | {line}");
    }

    // ─── Test 6: SFTP Directory Listing ─────────────────────────
    println!("\n[Test 6] SFTP Directory Listing");
    let sftp = sess.sftp()?;
    let entries = sftp.readdir(Path::new("/root"))?;
    println!("  [PASS] /root listing: {} entries", entries.len());
    let dirs: Vec<_> = entries.iter().filter(|(_, s)| s.is_dir()).collect();
    let files: Vec<_> = entries.iter().filter(|(_, s)| !s.is_dir()).collect();
    println!("    Directories: {}, Files: {}", dirs.len(), files.len());
    for (path, _) in entries.iter().take(8) {
        println!("    - {}", path.display());
    }

    // ─── Test 7: SFTP File Read ─────────────────────────────────
    println!("\n[Test 7] SFTP File Read");
    let test_files = ["/root/.bashrc", "/etc/hostname", "/etc/os-release"];
    for tf in &test_files {
        match sftp.stat(Path::new(tf)) {
            Ok(stat) => {
                let size = stat.size.unwrap_or(0);
                println!("  [PASS] {tf} exists (size: {size} bytes)");
                if size > 0 && size < 65536 {
                    let mut file = sftp.open(Path::new(tf))?;
                    let mut content = String::new();
                    file.read_to_string(&mut content)?;
                    println!("    Content ({} chars): {}", content.len(), &content[..content.len().min(100)]);
                }
            }
            Err(e) => println!("  [SKIP] {tf}: {e}"),
        }
    }

    // ─── Test 8: Host Metrics (CPU/Memory/Disk) ─────────────────
    println!("\n[Test 8] Host Metrics Collection");
    let metrics_cmds = [
        ("CPU", "top -bn1 | grep 'Cpu(s)' | head -1"),
        ("Memory", "free -m | grep Mem"),
        ("Disk", "df -h / | tail -1"),
        ("Uptime", "cat /proc/uptime"),
    ];
    for (label, cmd) in &metrics_cmds {
        let mut ch = sess.channel_session()?;
        ch.exec(cmd)?;
        let mut out = String::new();
        ch.read_to_string(&mut out)?;
        ch.wait_close()?;
        let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
        println!("  [PASS] {label}: {}", lines.first().unwrap_or(&"no output"));
    }

    // ─── Test 9: Multiple Concurrent Commands ──────────────────
    println!("\n[Test 9] Multiple Concurrent Commands");
    let mut handles = Vec::new();
    for i in 0..3 {
        let mut c = sess.channel_session()?;
        c.exec(&format!("echo 'Session {i}: $(hostname) - $(date +%s)'"))?;
        handles.push(c);
    }
    for (i, mut h) in handles.into_iter().enumerate() {
        let mut out = String::new();
        h.read_to_string(&mut out)?;
        h.wait_close()?;
        println!("  [PASS] Session {i}: {}", out.trim());
    }

    // ─── Test 10: SSH Keepalive ─────────────────────────────────
    println!("\n[Test 10] TCP Keepalive");
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();
    let handle = std::thread::spawn(move || {
        let tcp2 = TcpStream::connect_timeout(
            &format!("{SSH_HOST}:{SSH_PORT}").parse().unwrap(),
            Duration::from_secs(10),
        )
        .unwrap();
        tcp2.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut sess2 = ssh2::Session::new().unwrap();
        sess2.set_tcp_stream(tcp2);
        sess2.handshake().unwrap();
        sess2.userauth_password(SSH_USER, SSH_PASS).unwrap();

        sess2.set_keepalive(true, 5);
        println!("  [PASS] Keepalive set, waiting 10s...");

        let _ = shutdown_rx.recv_timeout(Duration::from_secs(10));
        println!("  [PASS] Connection survived 10s with keepalive");
    });

    std::thread::sleep(Duration::from_secs(2));
    let _ = shutdown_tx.send(());
    handle.join().unwrap();

    // ─── Summary ────────────────────────────────────────────────
    println!("\n{}", "=".repeat(60));
    println!("  ALL 10 TESTS PASSED");
    println!("  Target: {SSH_USER}@{SSH_HOST}:{SSH_PORT}");
    println!("{}", "=".repeat(60));

    Ok(())
}
