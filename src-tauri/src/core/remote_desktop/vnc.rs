//! VNC (RFB protocol) client implementation.
//!
//! Implements the RFB 3.3 handshake, VNC authentication, framebuffer update
//! polling (Raw encoding), and input event forwarding.

use std::collections::VecDeque;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::desktop_client::DesktopClient;
use super::frame_encoder::FrameEncoder;
use crate::types::remote_desktop::frame::DesktopFrame;
use crate::types::remote_desktop::input::{KeyEvent, MouseEvent};
use crate::types::remote_desktop::session::DesktopSession;

/// 4 bytes per pixel (R, G, B, padding)
const BPP: usize = 4;

pub struct VncClient {
    session_id: String,
    stream: Option<tokio::net::TcpStream>,
    width: u16,
    height: u16,
    /// Raw framebuffer, 4 bytes per pixel (R, G, B, pad)
    framebuffer: Vec<u8>,
    encoder: FrameEncoder,
    /// Queue of pending tile frames to return across poll_frame calls
    pending_frames: VecDeque<DesktopFrame>,
    /// Whether the initial full frame has been requested
    first_update: bool,
}

impl VncClient {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            stream: None,
            width: 0,
            height: 0,
            framebuffer: Vec::new(),
            encoder: FrameEncoder::new(0, 0, 75),
            pending_frames: VecDeque::new(),
            first_update: true,
        }
    }
}

#[async_trait]
impl DesktopClient for VncClient {
    async fn connect(&mut self, session: &DesktopSession, password: &str) -> Result<(), String> {
        let addr = format!("{}:{}", session.host, session.port);
        let mut stream = tokio::net::TcpStream::connect(&addr)
            .await
            .map_err(|e| format!("VNC connect to {addr} failed: {e}"))?;

        // ── 1. ProtocolVersion handshake ──
        // Read server version (12 bytes)
        let mut srv_ver = [0u8; 12];
        stream
            .read_exact(&mut srv_ver)
            .await
            .map_err(|e| format!("VNC read ProtocolVersion: {e}"))?;

        // Negotiate to RFB 3.3 for simple security handshake
        stream
            .write_all(b"RFB 003.003\n")
            .await
            .map_err(|e| format!("VNC write ProtocolVersion: {e}"))?;

        // ── 2. Security handshake (RFB 3.3: server sends 4-byte type) ──
        let mut sec_buf = [0u8; 4];
        stream
            .read_exact(&mut sec_buf)
            .await
            .map_err(|e| format!("VNC read SecurityType: {e}"))?;
        let sec_type = u32::from_be_bytes(sec_buf);

        match sec_type {
            0 => {
                // Connection failed; read error reason
                let mut err_len_buf = [0u8; 4];
                stream
                    .read_exact(&mut err_len_buf)
                    .await
                    .map_err(|e| format!("VNC read error length: {e}"))?;
                let err_len = u32::from_be_bytes(err_len_buf);
                let mut err_msg = vec![0u8; err_len as usize];
                if !err_msg.is_empty() {
                    stream
                        .read_exact(&mut err_msg)
                        .await
                        .map_err(|e| format!("VNC read error msg: {e}"))?;
                }
                return Err(format!(
                    "VNC server refused connection: {}",
                    String::from_utf8_lossy(&err_msg)
                ));
            }
            1 => {
                // None — no authentication needed
            }
            2 => {
                // VNC Authentication
                let mut challenge = [0u8; 16];
                stream
                    .read_exact(&mut challenge)
                    .await
                    .map_err(|e| format!("VNC read challenge: {e}"))?;

                let response = vnc_encrypt_challenge(password, &challenge);
                stream
                    .write_all(&response)
                    .await
                    .map_err(|e| format!("VNC write challenge response: {e}"))?;
            }
            other => {
                return Err(format!("VNC unsupported security type: {other}"));
            }
        }

        // ── 3. SecurityResult ──
        let mut result_buf = [0u8; 4];
        stream
            .read_exact(&mut result_buf)
            .await
            .map_err(|e| format!("VNC read SecurityResult: {e}"))?;
        let result = u32::from_be_bytes(result_buf);
        if result != 0 {
            return Err(format!("VNC authentication failed (result={result})"));
        }

        // ── 4. ClientInit ──
        // shared-flag = 1 (non-exclusive access)
        stream
            .write_all(&[1u8])
            .await
            .map_err(|e| format!("VNC write ClientInit: {e}"))?;

        // ── 5. ServerInit ──
        // fb-width(2) + fb-height(2) + pixel-format(16) + name-length(4)
        let mut init_header = [0u8; 24];
        stream
            .read_exact(&mut init_header)
            .await
            .map_err(|e| format!("VNC read ServerInit header: {e}"))?;

        let width = u16::from_be_bytes([init_header[0], init_header[1]]);
        let height = u16::from_be_bytes([init_header[2], init_header[3]]);

        // Extract name length (last 4 bytes of the 24-byte header)
        let name_len =
            u32::from_be_bytes([init_header[20], init_header[21], init_header[22], init_header[23]]);
        let mut name_buf = vec![0u8; name_len as usize];
        if name_len > 0 {
            stream
                .read_exact(&mut name_buf)
                .await
                .map_err(|e| format!("VNC read desktop name: {e}"))?;
        }

        self.width = width;
        self.height = height;
        self.framebuffer = vec![0u8; width as usize * height as usize * BPP];
        self.encoder = FrameEncoder::new(width, height, session.quality);
        self.stream = Some(stream);

        Ok(())
    }

    async fn poll_frame(&mut self) -> Result<Option<DesktopFrame>, String> {
        // Return queued tiles first
        if let Some(frame) = self.pending_frames.pop_front() {
            return Ok(Some(frame));
        }

        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "VNC not connected".to_string())?;

        // ── Send FramebufferUpdateRequest ──
        // msg-type(1) + incremental(1) + x(2) + y(2) + w(2) + h(2) = 10 bytes
        let mut req = [0u8; 10];
        req[0] = 3; // FramebufferUpdateRequest
        req[1] = if self.first_update { 0u8 } else { 1u8 }; // incremental
        req[6..8].copy_from_slice(&self.width.to_be_bytes());
        req[8..10].copy_from_slice(&self.height.to_be_bytes());
        stream
            .write_all(&req)
            .await
            .map_err(|e| format!("VNC write FBUR: {e}"))?;
        self.first_update = false;

        // ── Read server message ──
        self.read_framebuffer_update().await?;

        // ── Encode changed tiles ──
        let mut tiles = self.encoder.encode_frame(&self.framebuffer);
        for tile in tiles.iter_mut() {
            tile.session_id = self.session_id.clone();
        }

        // Queue all tiles, return the first one
        for tile in tiles {
            self.pending_frames.push_back(tile);
        }

        Ok(self.pending_frames.pop_front())
    }

    async fn send_key_event(&mut self, event: KeyEvent) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "VNC not connected".to_string())?;
        // msg-type(1) + down-flag(1) + padding(2) + key(4) = 8 bytes
        let mut buf = [0u8; 8];
        buf[0] = 4; // KeyEvent
        buf[1] = if event.pressed { 1 } else { 0 };
        buf[6..8].copy_from_slice(&event.key_code.to_be_bytes()[2..4]);
        buf[4..8].copy_from_slice(&event.key_code.to_be_bytes());
        stream
            .write_all(&buf)
            .await
            .map_err(|e| format!("VNC write KeyEvent: {e}"))?;
        Ok(())
    }

    async fn send_mouse_event(&mut self, event: MouseEvent) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "VNC not connected".to_string())?;
        // msg-type(1) + button-mask(1) + x(2) + y(2) = 6 bytes
        let button_mask = if event.pressed {
            1u8 << event.button
        } else {
            0u8
        };
        let mut buf = [0u8; 6];
        buf[0] = 5; // PointerEvent
        buf[1] = button_mask;
        buf[2..4].copy_from_slice(&event.x.to_be_bytes());
        buf[4..6].copy_from_slice(&event.y.to_be_bytes());
        stream
            .write_all(&buf)
            .await
            .map_err(|e| format!("VNC write PointerEvent: {e}"))?;
        Ok(())
    }

    async fn resize(&mut self, _width: u16, _height: u16) -> Result<(), String> {
        // VNC is server-driven; resize is a no-op.
        Ok(())
    }

    async fn push_clipboard(&mut self, text: String) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "VNC not connected".to_string())?;

        let text_bytes = text.as_bytes();
        let len = text_bytes.len();
        if len > 1_048_576 {
            return Err("VNC clipboard text too large".to_string());
        }

        let mut buf = Vec::with_capacity(8 + len);
        buf.push(6); // msg-type: ClientCutText
        buf.extend_from_slice(&[0u8; 3]); // padding
        buf.extend_from_slice(&(len as u32).to_be_bytes()); // text-length
        buf.extend_from_slice(text_bytes); // text

        stream
            .write_all(&buf)
            .await
            .map_err(|e| format!("VNC write ClientCutText: {e}"))
    }

    async fn disconnect(&mut self) -> Result<(), String> {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.shutdown().await;
        }
        self.pending_frames.clear();
        Ok(())
    }

    fn framebuffer_width(&self) -> u16 {
        self.width
    }

    fn framebuffer_height(&self) -> u16 {
        self.height
    }
}

impl VncClient {
    /// Read a FramebufferUpdate (message type 0) or other server messages from the stream.
    async fn read_framebuffer_update(&mut self) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "VNC not connected".to_string())?;

        // Read message type
        let mut msg_type = [0u8; 1];
        stream
            .read_exact(&mut msg_type)
            .await
            .map_err(|e| format!("VNC read msg type: {e}"))?;

        match msg_type[0] {
            0 => {
                // FramebufferUpdate
                // padding(1) + num-rects(2)
                let mut fb_header = [0u8; 3];
                stream
                    .read_exact(&mut fb_header)
                    .await
                    .map_err(|e| format!("VNC read FB header: {e}"))?;
                let num_rects = u16::from_be_bytes([fb_header[1], fb_header[2]]);

                for _ in 0..num_rects {
                    self.read_rectangle().await?;
                }
            }
            1 => {
                // SetColorMapEntries — read and ignore
                let mut pad = [0u8; 1];
                stream.read_exact(&mut pad).await.ok();
                let mut ignore = [0u8; 4 + 2]; // first-color(2) + num-colors(2)
                stream.read_exact(&mut ignore).await.ok();
                // We would need to read color entries, skip for now
            }
            2 => {
                // Bell — ignore
            }
            3 => {
                // ServerCutText
                let mut pad = [0u8; 3];
                stream.read_exact(&mut pad).await.ok();
                let mut len_buf = [0u8; 4];
                stream.read_exact(&mut len_buf).await.ok();
                let len = u32::from_be_bytes(len_buf);
                if len > 0 && len < 1024 * 1024 {
                    let mut text = vec![0u8; len as usize];
                    let _ = stream.read_exact(&mut text).await;
                }
            }
            other => {
                // Unknown message type — skip
                eprintln!("[azurepath] VNC unknown message type: {other}");
            }
        }

        Ok(())
    }

    /// Read a single rectangle from a FramebufferUpdate.
    async fn read_rectangle(&mut self) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "VNC not connected".to_string())?;

        // rect-header: x(2) + y(2) + w(2) + h(2) + encoding(4) = 12 bytes
        let mut header = [0u8; 12];
        stream
            .read_exact(&mut header)
            .await
            .map_err(|e| format!("VNC read rect header: {e}"))?;

        let rx = u16::from_be_bytes([header[0], header[1]]);
        let ry = u16::from_be_bytes([header[2], header[3]]);
        let rw = u16::from_be_bytes([header[4], header[5]]);
        let rh = u16::from_be_bytes([header[6], header[7]]);
        let encoding = i32::from_be_bytes([header[8], header[9], header[10], header[11]]);

        match encoding {
            0 => {
                // Raw encoding
                self.read_raw_rect(rx, ry, rw, rh).await?;
            }
            -239 => {
                // JPEG encoding (desktop area, not pixel data)
                // Read and decode JPEG rectangle
                self.read_jpeg_rect(rx, ry, rw, rh).await?;
            }
            _ => {
                // Unsupported encoding — skip the pixel data
                // Estimate pixel data size (worst case: 4 bytes per pixel)
                let skip_bytes = rw as usize * rh as usize * BPP;
                let mut skip = vec![0u8; skip_bytes.min(4 * 1024 * 1024)];
                let mut remaining = skip_bytes;
                while remaining > 0 {
                    let to_read = remaining.min(skip.len());
                    let mut buf = &mut skip[..to_read];
                    match stream.read_exact(&mut buf).await {
                        Ok(_) => remaining -= to_read,
                        Err(e) => {
                            return Err(format!("VNC skip rect data: {e}"));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Read a Raw-encoded rectangle and write into the framebuffer.
    async fn read_raw_rect(&mut self, rx: u16, ry: u16, rw: u16, rh: u16) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "VNC not connected".to_string())?;

        let pixel_count = rw as usize * rh as usize;
        let raw_size = pixel_count * BPP;
        let mut raw = vec![0u8; raw_size];
        stream
            .read_exact(&mut raw)
            .await
            .map_err(|e| format!("VNC read raw rect: {e}"))?;

        // Copy pixels into framebuffer at (rx, ry)
        let fb_w = self.width as usize;
        for row in 0..rh as usize {
            for col in 0..rw as usize {
                let src_idx = (row * rw as usize + col) * BPP;
                let dst_idx = ((ry as usize + row) * fb_w + (rx as usize + col)) * BPP;
                if dst_idx + BPP <= self.framebuffer.len() {
                    self.framebuffer[dst_idx..dst_idx + BPP]
                        .copy_from_slice(&raw[src_idx..src_idx + BPP]);
                }
            }
        }

        Ok(())
    }

    /// Read a JPEG-encoded rectangle (encoding type -239) and update the framebuffer.
    async fn read_jpeg_rect(&mut self, rx: u16, ry: u16, rw: u16, rh: u16) -> Result<(), String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "VNC not connected".to_string())?;

        // JPEG rectangles in TightVNC have the JPEG data inline.
        // Read data-length(4) + jpeg-data
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| format!("VNC read JPEG data length: {e}"))?;
        let data_len = u32::from_be_bytes(len_buf) as usize;

        if data_len > 10 * 1024 * 1024 {
            return Err(format!("VNC JPEG rect too large: {data_len}"));
        }

        let mut jpeg_data = vec![0u8; data_len];
        if data_len > 0 {
            stream
                .read_exact(&mut jpeg_data)
                .await
                .map_err(|e| format!("VNC read JPEG data: {e}"))?;
        }

        // Decode JPEG and update framebuffer
        if let Ok(img) = image::load_from_memory(&jpeg_data) {
            let rgb = img.to_rgb8();
            let fb_w = self.width as usize;
            for row in 0..rh as usize {
                for col in 0..rw as usize {
                    let pixel = rgb.get_pixel(col as u32, row as u32);
                    let dst_idx = ((ry as usize + row) * fb_w + (rx as usize + col)) * BPP;
                    if dst_idx + BPP <= self.framebuffer.len() {
                        self.framebuffer[dst_idx] = pixel[0]; // R
                        self.framebuffer[dst_idx + 1] = pixel[1]; // G
                        self.framebuffer[dst_idx + 2] = pixel[2]; // B
                        // padding (byte 3) stays as-is
                    }
                }
            }
        }

        Ok(())
    }
}

// ── VNC Authentication: DES challenge/response ──

/// Encrypt a 16-byte VNC challenge using the password.
///
/// VNC authentication uses DES in ECB mode with a key derived from the password:
/// 1. Pad or truncate password to 8 bytes.
/// 2. Reverse the bits of each byte.
/// 3. Use as DES key to encrypt the 16-byte challenge (two 8-byte blocks).
fn vnc_encrypt_challenge(password: &str, challenge: &[u8]) -> Vec<u8> {
    use cipher::{BlockEncrypt, KeyInit};
    use des::Des;

    // Build 8-byte key with bit-reversed password bytes
    let mut key_bytes = [0u8; 8];
    for (i, b) in password.bytes().enumerate().take(8) {
        key_bytes[i] = b.reverse_bits();
    }

    let key = cipher::generic_array::GenericArray::from_slice(&key_bytes);
    let cipher = Des::new(key);

    let mut result = Vec::with_capacity(16);
    for chunk in challenge.chunks(8) {
        let mut block = cipher::generic_array::GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        result.extend_from_slice(&block);
    }
    result
}
