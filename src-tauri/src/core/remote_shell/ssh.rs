//! SSH 终端实现：使用 ssh2 (libssh2) 库在独立线程中运行 worker，
//! 通过 channel 与异步主循环交换数据。

use async_trait::async_trait;
use std::io::Read;
use std::io::Write;
use std::sync::mpsc as std_mpsc;
use std::thread;
use std::time::Duration;

use crate::types::remote_shell::session::RemoteSession;
use crate::types::remote_shell::terminal::{TerminalClient, TerminalError};

enum WorkerCommand {
    Write(Vec<u8>),
    Resize(u16, u16),
    Disconnect,
}

pub struct SshClient {
    cmd_tx: Option<std_mpsc::Sender<WorkerCommand>>,
    output_rx: Option<std_mpsc::Receiver<Vec<u8>>>,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
}

impl SshClient {
    pub fn new(password: Option<String>) -> Self {
        Self { cmd_tx: None, output_rx: None, host: None, port: None, username: None, password }
    }

    fn launch_worker(&mut self, host: String, port: u16, username: String, password: String) -> Result<(), TerminalError> {
        let (cmd_tx, cmd_rx) = std_mpsc::channel::<WorkerCommand>();
        let (output_tx, output_rx) = std_mpsc::channel::<Vec<u8>>();

        thread::spawn(move || {
            let addr = format!("{}:{}", host, port);
            match std::net::TcpStream::connect(&addr) {
                Ok(tcp) => {
                    tcp.set_read_timeout(Some(Duration::from_millis(50))).ok();
                    let mut sess = match ssh2::Session::new() {
                        Ok(s) => s,
                        Err(e) => {
                            let msg = format!("\r\nSSH session error: {}\r\n", e);
                            output_tx.send(msg.into_bytes()).ok();
                            return;
                        }
                    };
                    sess.set_tcp_stream(tcp);
                    if sess.handshake().is_err() {
                        output_tx.send(b"\r\nSSH handshake failed\r\n".to_vec()).ok();
                        return;
                    }
                    if sess.userauth_password(&username, &password).is_err() {
                        output_tx.send(b"\r\nSSH authentication failed\r\n".to_vec()).ok();
                        return;
                    }
                    match sess.channel_session() {
                        Ok(mut channel) => {
                            channel.request_pty("xterm-256color", None, Some((80, 24, 0, 0))).ok();
                            channel.shell().ok();
                            let mut buf = [0u8; 8192];
                            loop {
                                // Drain commands
                                loop {
                                    match cmd_rx.try_recv() {
                                        Ok(WorkerCommand::Write(data)) => {
                                            channel.write_all(&data).ok();
                                            channel.flush().ok();
                                        }
                                        Ok(WorkerCommand::Resize(cols, rows)) => {
                                            channel.request_pty_size(cols as u32, rows as u32, None, None).ok();
                                        }
                                        Ok(WorkerCommand::Disconnect) => {
                                            channel.close().ok();
                                            channel.wait_close().ok();
                                            return;
                                        }
                                        Err(std_mpsc::TryRecvError::Disconnected) => {
                                            return;
                                        }
                                        Err(std_mpsc::TryRecvError::Empty) => break,
                                    }
                                }
                                // Read output
                                match channel.read(&mut buf) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        output_tx.send(buf[..n].to_vec()).ok();
                                    }
                                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock
                                        || e.kind() == std::io::ErrorKind::TimedOut =>
                                    {
                                        thread::sleep(Duration::from_millis(10));
                                    }
                                    Err(_) => break,
                                }
                            }
                            channel.close().ok();
                            channel.wait_close().ok();
                        }
                        Err(e) => {
                            let msg = format!("\r\nChannel error: {}\r\n", e);
                            output_tx.send(msg.into_bytes()).ok();
                        }
                    }
                }
                Err(e) => {
                    let msg = format!("\r\nTCP connect failed: {}\r\n", e);
                    output_tx.send(msg.into_bytes()).ok();
                }
            }
        });

        self.cmd_tx = Some(cmd_tx);
        self.output_rx = Some(output_rx);
        Ok(())
    }
}

#[async_trait]
impl TerminalClient for SshClient {
    async fn connect(&mut self, session: &RemoteSession, password: &str) -> Result<(), TerminalError> {
        self.host = Some(session.host.clone());
        self.port = Some(session.port);
        self.username = Some(session.username.clone());
        self.password = Some(password.to_string());
        self.launch_worker(
            session.host.clone(),
            session.port,
            session.username.clone(),
            password.to_string(),
        )
    }

    async fn read(&mut self) -> Result<Vec<u8>, TerminalError> {
        let rx = self.output_rx.as_mut().ok_or(TerminalError::SessionNotFound)?;
        let mut merged = Vec::new();
        for _ in 0..64 {
            match rx.try_recv() {
                Ok(chunk) => merged.extend_from_slice(&chunk),
                Err(_) => break,
            }
        }
        if merged.is_empty() {
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(data) => Ok(data),
                Err(_) => Ok(vec![]),
            }
        } else {
            Ok(merged)
        }
    }

    async fn write(&mut self, data: &[u8]) -> Result<(), TerminalError> {
        let tx = self.cmd_tx.as_ref().ok_or(TerminalError::SessionNotFound)?;
        tx.send(WorkerCommand::Write(data.to_vec()))
            .map_err(|_| TerminalError::Io("channel closed".to_string()))
    }

    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TerminalError> {
        if let Some(tx) = self.cmd_tx.as_ref() {
            tx.send(WorkerCommand::Resize(cols, rows)).ok();
        }
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), TerminalError> {
        if let Some(tx) = self.cmd_tx.take() {
            tx.send(WorkerCommand::Disconnect).ok();
        }
        self.output_rx = None;
        Ok(())
    }
}
