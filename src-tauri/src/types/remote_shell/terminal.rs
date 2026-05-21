use async_trait::async_trait;
use serde::Serialize;
use thiserror::Error;

use super::session::RemoteSession;

#[derive(Debug, Error, Serialize)]
#[allow(dead_code)]
pub enum TerminalError {
    #[error("connection failed: {0}")]
    Connection(String),
    #[error("io failed: {0}")]
    Io(String),
    #[error("session not found")]
    SessionNotFound,
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("protocol not supported: {0}")]
    UnsupportedProtocol(String),
}

#[async_trait]
pub trait TerminalClient: Send {
    async fn connect(&mut self, session: &RemoteSession, password: &str) -> Result<(), TerminalError>;
    async fn read(&mut self) -> Result<Vec<u8>, TerminalError>;
    async fn write(&mut self, data: &[u8]) -> Result<(), TerminalError>;
    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TerminalError>;
    async fn disconnect(&mut self) -> Result<(), TerminalError>;
}
