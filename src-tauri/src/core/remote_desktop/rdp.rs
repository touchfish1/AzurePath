//! RDP client implementation using IronRDP.
//!
//! Architecture:
//! - Blocking I/O via `ironrdp_blocking::Framed` on `std::net::TcpStream`
//! - Connection and active-stage processing run on a dedicated OS thread
//! - `mpsc` channels carry commands (keyboard / mouse / resize / disconnect) from the
//!   async frontend to the blocking worker, and encoded JPEG tiles back
//! - `FrameEncoder` performs delta-based tile comparison and JPEG compression

use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use async_trait::async_trait;
use ironrdp::connector;
use ironrdp::connector::Credentials;
use ironrdp::graphics::image_processing::PixelFormat;
use ironrdp::pdu::gcc::KeyboardType;
use ironrdp::pdu::input::fast_path::{FastPathInput, FastPathInputEvent, KeyboardFlags};
use ironrdp::pdu::input::mouse::PointerFlags;
use ironrdp::pdu::input::MousePdu;
use ironrdp::pdu::rdp::client_info::{PerformanceFlags, TimezoneInfo};
use ironrdp::session::image::DecodedImage;
use ironrdp::session::{ActiveStage, ActiveStageOutput};
use ironrdp::core::{Encode, WriteCursor};
use sspi::network_client::reqwest_network_client::ReqwestNetworkClient;
use tokio_rustls::rustls;
use x509_cert::der::Decode as _;
use x509_cert::Certificate;

use super::desktop_client::DesktopClient;
use super::frame_encoder::FrameEncoder;
use crate::types::remote_desktop::frame::DesktopFrame;
use crate::types::remote_desktop::input::{KeyEvent, MouseEvent};
use crate::types::remote_desktop::session::DesktopSession;

// ── Command channel ──

/// Commands sent from the async frontend to the blocking RDP worker thread.
enum RdpCommand {
    KeyEvent(KeyEvent),
    MouseEvent(MouseEvent),
    Resize(u16, u16),
    Disconnect,
}

/// TLS-upgraded framed stream type alias.
type UpgradedFramed = ironrdp_blocking::Framed<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>;

// ── Public client ──

pub struct RdpClient {
    session_id: String,
    cmd_tx: Option<mpsc::Sender<RdpCommand>>,
    frame_rx: Option<mpsc::Receiver<DesktopFrame>>,
    thread_handle: Option<thread::JoinHandle<()>>,
    width: u16,
    height: u16,
}

impl RdpClient {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            cmd_tx: None,
            frame_rx: None,
            thread_handle: None,
            width: 0,
            height: 0,
        }
    }
}

#[async_trait]
impl DesktopClient for RdpClient {
    async fn connect(&mut self, session: &DesktopSession, password: &str) -> Result<(), String> {
        let session = session.clone();
        let password = password.to_string();
        let sid = self.session_id.clone();

        // Spawn a blocking task for the entire RDP connection handshake (TCP + TLS + CredSSP).
        // Returns the channel endpoints and worker thread handle.
        let (cmd_tx, frame_rx, handle, width, height) = tokio::task::spawn_blocking(move || {
            blocking_connect(&sid, &session, &password)
        })
        .await
        .map_err(|e| format!("RDP spawn blocking failed: {e}"))?
        .map_err(|e| format!("RDP connect error: {e}"))?;

        self.cmd_tx = Some(cmd_tx);
        self.frame_rx = Some(frame_rx);
        self.thread_handle = Some(handle);
        self.width = width;
        self.height = height;

        Ok(())
    }

    async fn poll_frame(&mut self) -> Result<Option<DesktopFrame>, String> {
        let rx = self
            .frame_rx
            .as_mut()
            .ok_or_else(|| "RDP not connected".to_string())?;
        match rx.try_recv() {
            Ok(frame) => Ok(Some(frame)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                self.thread_handle.take();
                Err("RDP connection lost".to_string())
            }
        }
    }

    async fn send_key_event(&mut self, event: KeyEvent) -> Result<(), String> {
        let tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| "RDP not connected".to_string())?;
        tx.send(RdpCommand::KeyEvent(event))
            .map_err(|e| format!("RDP send command failed: {e}"))
    }

    async fn send_mouse_event(&mut self, event: MouseEvent) -> Result<(), String> {
        let tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| "RDP not connected".to_string())?;
        tx.send(RdpCommand::MouseEvent(event))
            .map_err(|e| format!("RDP send command failed: {e}"))
    }

    async fn resize(&mut self, width: u16, height: u16) -> Result<(), String> {
        let tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| "RDP not connected".to_string())?;
        tx.send(RdpCommand::Resize(width, height))
            .map_err(|e| format!("RDP send command failed: {e}"))
    }

    async fn push_clipboard(&mut self, text: String) -> Result<(), String> {
        let tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| "RDP not connected".to_string())?;

        // Send Ctrl+V keystroke sequence to paste the text.
        // Note: Proper cliprdr virtual channel support would require
        // `ironrdp-cliprdr-native` integration; for now, simulation via
        // keyboard shortcut is a practical fallback for an intranet tool.
        let ctrl = 17u32;
        let v_key = 86u32;

        tx.send(RdpCommand::KeyEvent(KeyEvent {
            key_code: ctrl,
            pressed: true,
        }))
        .map_err(|e| format!("RDP clipboard send failed: {e}"))?;

        tx.send(RdpCommand::KeyEvent(KeyEvent {
            key_code: v_key,
            pressed: true,
        }))
        .map_err(|e| format!("RDP clipboard send failed: {e}"))?;

        // Small delay to let the server process the keys
        tokio::time::sleep(Duration::from_millis(50)).await;

        tx.send(RdpCommand::KeyEvent(KeyEvent {
            key_code: v_key,
            pressed: false,
        }))
        .map_err(|e| format!("RDP clipboard send failed: {e}"))?;

        tx.send(RdpCommand::KeyEvent(KeyEvent {
            key_code: ctrl,
            pressed: false,
        }))
        .map_err(|e| format!("RDP clipboard send failed: {e}"))
    }

    async fn disconnect(&mut self) -> Result<(), String> {
        // Signal the worker thread to stop
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.send(RdpCommand::Disconnect);
        }
        // Wait for the thread to finish
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        self.frame_rx.take();
        Ok(())
    }

    fn framebuffer_width(&self) -> u16 {
        self.width
    }

    fn framebuffer_height(&self) -> u16 {
        self.height
    }
}

// ── Blocking connection helper ──

/// Performs the entire blocking RDP connection sequence and spawns the worker
/// thread, returning the channel endpoints and thread handle.
fn blocking_connect(
    session_id: &str,
    session: &DesktopSession,
    password: &str,
) -> Result<
    (
        mpsc::Sender<RdpCommand>,
        mpsc::Receiver<DesktopFrame>,
        thread::JoinHandle<()>,
        u16,
        u16,
    ),
    String,
> {
    let addr = format!("{}:{}", session.host, session.port);
    let tcp_stream =
        TcpStream::connect(&addr).map_err(|e| format!("RDP TCP connect to {addr} failed: {e}"))?;

    // Set read timeout so the active loop can periodically check for commands
    tcp_stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|e| format!("RDP set read timeout failed: {e}"))?;

    let client_addr = tcp_stream
        .local_addr()
        .map_err(|e| format!("RDP get local addr failed: {e}"))?;

    // ── Build config ──
    let config = build_config(session, password);

    // ── Phase 1: connect begin ──
    let mut framed = ironrdp_blocking::Framed::new(tcp_stream);
    let mut connector = connector::ClientConnector::new(config, client_addr);
    let should_upgrade = ironrdp_blocking::connect_begin(&mut framed, &mut connector)
        .map_err(|e| format!("RDP connect begin failed: {e}"))?;

    // ── Phase 2: TLS upgrade ──
    let initial_stream = framed.into_inner_no_leftover();
    let (upgraded_stream, server_public_key) =
        tls_upgrade(initial_stream, &session.host).map_err(|e| format!("RDP TLS upgrade: {e}"))?;

    let upgraded = ironrdp_blocking::mark_as_upgraded(should_upgrade, &mut connector);
    let mut upgraded_framed = ironrdp_blocking::Framed::new(upgraded_stream);

    // ── Phase 3: CredSSP + finalize ──
    let mut network_client = ReqwestNetworkClient;
    let connection_result = ironrdp_blocking::connect_finalize(
        upgraded,
        connector,
        &mut upgraded_framed,
        &mut network_client,
        session.host.clone().into(),
        server_public_key,
        None,
    )
    .map_err(|e| format!("RDP connect finalize failed: {e}"))?;

    let width = connection_result.desktop_size.width;
    let height = connection_result.desktop_size.height;

    // ── Phase 4: prepare image buffer and active stage ──
    let image = DecodedImage::new(PixelFormat::RgbA32, width, height);
    let active_stage = ActiveStage::new(connection_result);

    // ── Phase 5: channel creation and worker spawn ──
    let (cmd_tx, cmd_rx) = mpsc::channel::<RdpCommand>();
    let (frame_tx, frame_rx) = mpsc::channel::<DesktopFrame>();

    let sid = session_id.to_string();
    let quality = session.quality;
    let handle = thread::Builder::new()
        .name("rdp-worker".into())
        .spawn(move || {
            rdp_worker(upgraded_framed, active_stage, image, cmd_rx, frame_tx, &sid, quality);
        })
        .map_err(|e| format!("RDP spawn worker thread: {e}"))?;

    Ok((cmd_tx, frame_rx, handle, width, height))
}

// ── Worker thread ──

/// Main loop running on the dedicated OS thread.
///
/// Reads RDP PDUs, feeds them into `ActiveStage`, encodes framebuffer changes as
/// JPEG tiles, and forwards those tiles to the async frontend via `frame_tx`.
/// Commands from the frontend (keyboard, mouse, resize, disconnect) arrive through
/// `cmd_rx` and are written directly into the framed stream.
fn rdp_worker(
    mut framed: UpgradedFramed,
    mut active_stage: ActiveStage,
    mut image: DecodedImage,
    cmd_rx: mpsc::Receiver<RdpCommand>,
    frame_tx: mpsc::Sender<DesktopFrame>,
    session_id: &str,
    quality: u8,
) {
    let fb_width = image.width();
    let fb_height = image.height();
    let mut encoder = FrameEncoder::new(fb_width, fb_height, quality);

    loop {
        // ── Drain pending commands ──
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                RdpCommand::KeyEvent(ev) => send_rdp_keyboard(&mut framed, &ev),
                RdpCommand::MouseEvent(ev) => send_rdp_mouse(&mut framed, &ev),
                RdpCommand::Resize(w, h) => encoder.resize(w, h),
                RdpCommand::Disconnect => return,
            }
        }

        // ── Read and process RDP PDUs ──
        let mut had_data = false;
        loop {
            match framed.read_pdu() {
                Ok((action, payload)) => {
                    had_data = true;
                    match active_stage.process(&mut image, action, &payload) {
                        Ok(outputs) => {
                            for out in outputs {
                                match out {
                                    ActiveStageOutput::ResponseFrame(frame) => {
                                        if framed.write_all(&frame).is_err() {
                                            eprintln!("[azurepath] RDP write response failed");
                                            return;
                                        }
                                    }
                                    ActiveStageOutput::Terminate(_) => {
                                        eprintln!("[azurepath] RDP session terminated by server");
                                        return;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[azurepath] RDP process error: {e}");
                            return;
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    eprintln!("[azurepath] RDP read PDU error: {e}");
                    return;
                }
            }
        }

        // ── Encode framebuffer changes ──
        if had_data {
            // Borrow the pixel data without consuming the image.
            // `DecodedImage::data()` returns `&[u8]` in RGBA 32-bit-per-pixel format.
            let raw = image.data();
            let tiles = encoder.encode_frame(raw);
            for mut tile in tiles {
                tile.session_id = session_id.to_string();
                if frame_tx.send(tile).is_err() {
                    return; // receiver dropped
                }
            }
        } else {
            // No data received this cycle; yield to avoid busy-waiting
            thread::sleep(Duration::from_millis(10));
        }
    }
}

// ── Config construction ──

fn build_config(session: &DesktopSession, password: &str) -> connector::Config {
    let desktop_size = connector::DesktopSize {
        width: session.desktop_width,
        height: session.desktop_height,
    };

    connector::Config {
        credentials: Credentials::UsernamePassword {
            username: session.username.clone(),
            password: password.to_owned(),
        },
        domain: session.domain.clone(),
        enable_tls: false, // We handle TLS manually in the connection sequence
        enable_credssp: true,
        keyboard_type: KeyboardType::IbmEnhanced,
        keyboard_subtype: 0,
        keyboard_layout: 0,
        keyboard_functional_keys_count: 12,
        ime_file_name: String::new(),
        dig_product_id: String::new(),
        desktop_size,
        bitmap: None,
        client_build: 0,
        client_name: "AzurePath".to_owned(),
        client_dir: "C:\\Windows\\System32\\mstscax.dll".to_owned(),
        platform: std::convert::identity({
            #[cfg(windows)]
            {
                ironrdp::pdu::rdp::capability_sets::MajorPlatformType::WINDOWS
            }
            #[cfg(target_os = "macos")]
            {
                ironrdp::pdu::rdp::capability_sets::MajorPlatformType::MACINTOSH
            }
            #[cfg(target_os = "linux")]
            {
                ironrdp::pdu::rdp::capability_sets::MajorPlatformType::UNIX
            }
        }),
        enable_server_pointer: false,
        request_data: None,
        autologon: false,
        enable_audio_playback: false,
        pointer_software_rendering: true,
        performance_flags: PerformanceFlags::default(),
        desktop_scale_factor: 0,
        hardware_id: None,
        license_cache: None,
        timezone_info: TimezoneInfo::default(),
    }
}

// ── TLS upgrade ──

/// Upgrades a raw TCP stream to TLS using rustls with no certificate verification.
fn tls_upgrade(
    stream: TcpStream,
    server_name: &str,
) -> Result<(rustls::StreamOwned<rustls::ClientConnection, TcpStream>, Vec<u8>), String> {
    let mut config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(std::sync::Arc::new(danger::NoCertificateVerification))
        .with_no_client_auth();

    // Disable TLS session resumption (not supported by CredSSP).
    config.resumption = rustls::client::Resumption::disabled();
    // Enable SSLKEYLOGFILE support for debugging.
    config.key_log = std::sync::Arc::new(rustls::KeyLogFile::new());

    let config = std::sync::Arc::new(config);
    let dns_name = rustls::pki_types::ServerName::try_from(server_name.to_owned())
        .map_err(|e| format!("invalid server name for TLS: {e}"))?;
    let client = rustls::ClientConnection::new(config, dns_name)
        .map_err(|e| format!("TLS ClientConnection: {e}"))?;

    let mut tls_stream = rustls::StreamOwned::new(client, stream);

    // Flush to drive the TLS handshake forward (ensures peer certificate is received).
    tls_stream
        .flush()
        .map_err(|e| format!("TLS flush (handshake): {e}"))?;

    let cert = tls_stream
        .conn
        .peer_certificates()
        .and_then(|certs| certs.first())
        .ok_or_else(|| "no peer certificate received".to_string())?;

    let server_public_key = extract_tls_server_public_key(cert)?;

    Ok((tls_stream, server_public_key))
}

/// Extract the DER-encoded subject public key from a TLS certificate.
fn extract_tls_server_public_key(cert_der: &[u8]) -> Result<Vec<u8>, String> {
    let cert =
        Certificate::from_der(cert_der).map_err(|e| format!("parse server certificate: {e}"))?;

    let public_key = cert
        .tbs_certificate
        .subject_public_key_info
        .subject_public_key
        .as_bytes()
        .ok_or_else(|| "subject public key BIT STRING is not aligned".to_string())?;

    Ok(public_key.to_owned())
}

// ── Input event helpers ──

// Helper: encode an ironrdp `FastPathInput` into `Vec<u8>` without requiring
// the `alloc` feature re-export from `ironrdp_core`.
fn encode_fastpath_input(input: &FastPathInput) -> Vec<u8> {
    let size = input.size();
    let mut buf = vec![0u8; size];
    let mut cursor = WriteCursor::new(&mut buf);
    // Buffer is guaranteed to be large enough (we asked for size()), so unwrap is safe.
    input.encode(&mut cursor).expect("FastPathInput encoding failed");
    buf
}

/// Construct and send a fast-path keyboard event to the RDP server.
fn send_rdp_keyboard(framed: &mut UpgradedFramed, event: &KeyEvent) {
    let Some(scancode) = keycode_to_scancode(event.key_code) else {
        return;
    };

    let mut flags = KeyboardFlags::empty();
    if !event.pressed {
        flags |= KeyboardFlags::RELEASE;
    }
    if is_extended_scancode(event.key_code) {
        flags |= KeyboardFlags::EXTENDED;
    }

    let input = FastPathInput::single(FastPathInputEvent::KeyboardEvent(flags, scancode));
    let encoded = encode_fastpath_input(&input);
    let _ = framed.write_all(&encoded);
}

/// Construct and send a fast-path mouse event to the RDP server.
fn send_rdp_mouse(framed: &mut UpgradedFramed, event: &MouseEvent) {
    let mut flags = PointerFlags::MOVE;

    if event.pressed {
        flags |= PointerFlags::DOWN;
    }

    flags |= match event.button {
        0 => PointerFlags::LEFT_BUTTON,
        1 => PointerFlags::MIDDLE_BUTTON_OR_WHEEL,
        2 => PointerFlags::RIGHT_BUTTON,
        _ => PointerFlags::empty(),
    };

    let wheel = match event.button {
        3 => 1,   // scroll up
        4 => -1,  // scroll down
        _ => 0,
    };

    if event.button == 3 || event.button == 4 {
        flags |= PointerFlags::VERTICAL_WHEEL;
        if event.button == 4 {
            flags |= PointerFlags::WHEEL_NEGATIVE;
        }
    }

    let mouse_pdu = MousePdu {
        flags,
        number_of_wheel_rotation_units: wheel,
        x_position: event.x,
        y_position: event.y,
    };

    let input = FastPathInput::single(FastPathInputEvent::MouseEvent(mouse_pdu));
    let encoded = encode_fastpath_input(&input);
    let _ = framed.write_all(&encoded);
}

/// Map JavaScript keyCode to RDP scan code (PC/AT compatible).
fn keycode_to_scancode(key_code: u32) -> Option<u8> {
    match key_code {
        8 => Some(0x0E),     // Backspace
        9 => Some(0x0F),     // Tab
        13 => Some(0x1C),    // Enter
        16 => Some(0x2A),    // Shift (left)
        17 => Some(0x1D),    // Ctrl (left)
        18 => Some(0x38),    // Alt (left)
        19 => Some(0x45),    // Pause
        20 => Some(0x3A),    // Caps Lock
        27 => Some(0x01),    // Escape
        32 => Some(0x39),    // Space
        33 => Some(0xC9),    // Page Up (extended)
        34 => Some(0xD1),    // Page Down (extended)
        35 => Some(0xCF),    // End (extended)
        36 => Some(0xC7),    // Home (extended)
        37 => Some(0xCB),    // Left (extended)
        38 => Some(0xC8),    // Up (extended)
        39 => Some(0xCD),    // Right (extended)
        40 => Some(0xD0),    // Down (extended)
        45 => Some(0xD2),    // Insert (extended)
        46 => Some(0xD3),    // Delete (extended)
        48 => Some(0x0B),    // 0
        49 => Some(0x02),    // 1
        50 => Some(0x03),    // 2
        51 => Some(0x04),    // 3
        52 => Some(0x05),    // 4
        53 => Some(0x06),    // 5
        54 => Some(0x07),    // 6
        55 => Some(0x08),    // 7
        56 => Some(0x09),    // 8
        57 => Some(0x0A),    // 9
        65 => Some(0x1E),    // A
        66 => Some(0x30),    // B
        67 => Some(0x2E),    // C
        68 => Some(0x20),    // D
        69 => Some(0x12),    // E
        70 => Some(0x21),    // F
        71 => Some(0x22),    // G
        72 => Some(0x23),    // H
        73 => Some(0x17),    // I
        74 => Some(0x24),    // J
        75 => Some(0x25),    // K
        76 => Some(0x26),    // L
        77 => Some(0x32),    // M
        78 => Some(0x31),    // N
        79 => Some(0x18),    // O
        80 => Some(0x19),    // P
        81 => Some(0x10),    // Q
        82 => Some(0x13),    // R
        83 => Some(0x1F),    // S
        84 => Some(0x14),    // T
        85 => Some(0x16),    // U
        86 => Some(0x2F),    // V
        87 => Some(0x11),    // W
        88 => Some(0x2D),    // X
        89 => Some(0x15),    // Y
        90 => Some(0x2C),    // Z
        96 => Some(0x52),    // Numpad 0
        97 => Some(0x4F),    // Numpad 1
        98 => Some(0x50),    // Numpad 2
        99 => Some(0x51),    // Numpad 3
        100 => Some(0x4B),   // Numpad 4
        101 => Some(0x4C),   // Numpad 5
        102 => Some(0x4D),   // Numpad 6
        103 => Some(0x47),   // Numpad 7
        104 => Some(0x48),   // Numpad 8
        105 => Some(0x49),   // Numpad 9
        106 => Some(0x37),   // Numpad * (extended)
        107 => Some(0x4E),   // Numpad +
        109 => Some(0x4A),   // Numpad -
        110 => Some(0x53),   // Numpad .
        111 => Some(0x35),   // Numpad / (extended)
        112 => Some(0x3B),   // F1
        113 => Some(0x3C),   // F2
        114 => Some(0x3D),   // F3
        115 => Some(0x3E),   // F4
        116 => Some(0x3F),   // F5
        117 => Some(0x40),   // F6
        118 => Some(0x41),   // F7
        119 => Some(0x42),   // F8
        120 => Some(0x43),   // F9
        121 => Some(0x44),   // F10
        122 => Some(0x57),   // F11
        123 => Some(0x58),   // F12
        144 => Some(0x45),   // Num Lock
        145 => Some(0x46),   // Scroll Lock
        186 => Some(0x27),   // ;:
        187 => Some(0x0D),   // =+
        188 => Some(0x33),   // ,<
        189 => Some(0x0C),   // -_
        190 => Some(0x34),   // .>
        191 => Some(0x35),   // /?
        192 => Some(0x29),   // `~
        219 => Some(0x1A),   // [{
        220 => Some(0x2B),   // \|
        221 => Some(0x1B),   // ]}
        222 => Some(0x28),   // '"
        _ => None,
    }
}

fn is_extended_scancode(key_code: u32) -> bool {
    matches!(key_code, 33..=40 | 45 | 46 | 106 | 111)
}

// ── TLS certificate verifier (dangerous: accepts all certificates) ──

mod danger {
    use tokio_rustls::rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
    use tokio_rustls::rustls::pki_types;
    use tokio_rustls::rustls::{DigitallySignedStruct, Error, SignatureScheme};

    #[derive(Debug)]
    pub(super) struct NoCertificateVerification;

    impl ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(
            &self,
            _: &pki_types::CertificateDer<'_>,
            _: &[pki_types::CertificateDer<'_>],
            _: &pki_types::ServerName<'_>,
            _: &[u8],
            _: pki_types::UnixTime,
        ) -> Result<ServerCertVerified, Error> {
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _: &[u8],
            _: &pki_types::CertificateDer<'_>,
            _: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _: &[u8],
            _: &pki_types::CertificateDer<'_>,
            _: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            vec![
                SignatureScheme::RSA_PKCS1_SHA1,
                SignatureScheme::ECDSA_SHA1_Legacy,
                SignatureScheme::RSA_PKCS1_SHA256,
                SignatureScheme::ECDSA_NISTP256_SHA256,
                SignatureScheme::RSA_PKCS1_SHA384,
                SignatureScheme::ECDSA_NISTP384_SHA384,
                SignatureScheme::RSA_PKCS1_SHA512,
                SignatureScheme::ECDSA_NISTP521_SHA512,
                SignatureScheme::RSA_PSS_SHA256,
                SignatureScheme::RSA_PSS_SHA384,
                SignatureScheme::RSA_PSS_SHA512,
                SignatureScheme::ED25519,
                SignatureScheme::ED448,
            ]
        }
    }
}
