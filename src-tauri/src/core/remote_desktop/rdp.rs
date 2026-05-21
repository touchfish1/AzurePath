//! RDP client stub — to be implemented in Phase 2.
//!
//! RDP requires the `rdlib` or `xrdp` crate. For now this is a placeholder
//! that implements the `DesktopClient` trait as a no-op.

use async_trait::async_trait;

use super::desktop_client::DesktopClient;
use crate::types::remote_desktop::frame::DesktopFrame;
use crate::types::remote_desktop::input::{KeyEvent, MouseEvent};
use crate::types::remote_desktop::session::DesktopSession;

pub struct RdpClient {
    session_id: String,
}

impl RdpClient {
    pub fn new(session_id: String) -> Self {
        Self { session_id }
    }
}

#[async_trait]
impl DesktopClient for RdpClient {
    async fn connect(&mut self, _session: &DesktopSession, _password: &str) -> Result<(), String> {
        Err("RDP support is not yet implemented".to_string())
    }

    async fn poll_frame(&mut self) -> Result<Option<DesktopFrame>, String> {
        Err("RDP support is not yet implemented".to_string())
    }

    async fn send_key_event(&mut self, _event: KeyEvent) -> Result<(), String> {
        Err("RDP support is not yet implemented".to_string())
    }

    async fn send_mouse_event(&mut self, _event: MouseEvent) -> Result<(), String> {
        Err("RDP support is not yet implemented".to_string())
    }

    async fn resize(&mut self, _width: u16, _height: u16) -> Result<(), String> {
        Err("RDP support is not yet implemented".to_string())
    }

    async fn disconnect(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn framebuffer_width(&self) -> u16 {
        0
    }

    fn framebuffer_height(&self) -> u16 {
        0
    }
}
