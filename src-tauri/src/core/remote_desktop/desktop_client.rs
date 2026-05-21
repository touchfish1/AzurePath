use async_trait::async_trait;

use crate::types::remote_desktop::frame::DesktopFrame;
use crate::types::remote_desktop::input::{KeyEvent, MouseEvent};
use crate::types::remote_desktop::session::DesktopSession;

#[async_trait]
pub trait DesktopClient: Send {
    async fn connect(&mut self, session: &DesktopSession, password: &str) -> Result<(), String>;
    async fn poll_frame(&mut self) -> Result<Option<DesktopFrame>, String>;
    async fn send_key_event(&mut self, event: KeyEvent) -> Result<(), String>;
    async fn send_mouse_event(&mut self, event: MouseEvent) -> Result<(), String>;
    async fn resize(&mut self, width: u16, height: u16) -> Result<(), String>;
    async fn disconnect(&mut self) -> Result<(), String>;
    async fn push_clipboard(&mut self, text: String) -> Result<(), String>;
    fn framebuffer_width(&self) -> u16;
    fn framebuffer_height(&self) -> u16;
}
