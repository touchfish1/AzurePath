//! Telnet 终端实现：基于 tokio::net::TcpStream 的简单读写。

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

use crate::types::remote_shell::session::RemoteSession;
use crate::types::remote_shell::terminal::{TerminalClient, TerminalError};

pub struct TelnetClient {
    stream: Option<TcpStream>,
}

impl TelnetClient {
    pub fn new() -> Self {
        Self { stream: None }
    }
}

#[async_trait]
impl TerminalClient for TelnetClient {
    async fn connect(&mut self, session: &RemoteSession, _password: &str) -> Result<(), TerminalError> {
        let target = format!("{}:{}", session.host, session.port);
        let stream = timeout(Duration::from_secs(5), TcpStream::connect(&target))
            .await
            .map_err(|_| TerminalError::Connection("telnet connect timeout".to_string()))?
            .map_err(|e| TerminalError::Connection(e.to_string()))?;
        self.stream = Some(stream);
        Ok(())
    }

    async fn read(&mut self) -> Result<Vec<u8>, TerminalError> {
        let stream = self.stream.as_mut().ok_or(TerminalError::SessionNotFound)?;
        let mut buf = vec![0u8; 4096];
        match timeout(Duration::from_millis(10), stream.read(&mut buf)).await {
            Ok(Ok(n)) => {
                buf.truncate(n);
                Ok(buf)
            }
            Ok(Err(e)) => Err(TerminalError::Io(e.to_string())),
            Err(_) => Ok(vec![]),
        }
    }

    async fn write(&mut self, data: &[u8]) -> Result<(), TerminalError> {
        let stream = self.stream.as_mut().ok_or(TerminalError::SessionNotFound)?;
        stream.write_all(data).await.map_err(|e| TerminalError::Io(e.to_string()))
    }

    async fn resize(&mut self, _cols: u16, _rows: u16) -> Result<(), TerminalError> {
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), TerminalError> {
        if let Some(mut stream) = self.stream.take() {
            // Graceful shutdown: send IAC GA + close
            let _ = stream.shutdown().await;
        }
        Ok(())
    }
}
