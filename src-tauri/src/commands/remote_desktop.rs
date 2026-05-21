//! Remote desktop Tauri commands — session management, VNC connect/disconnect,
//! input forwarding, and framebuffer streaming.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::core::remote_desktop::desktop_client::DesktopClient;
use crate::core::remote_desktop::rdp::RdpClient;
use crate::core::remote_desktop::session_store::DesktopSessionStore;
use crate::core::remote_desktop::vnc::VncClient;
use crate::types::remote_desktop::input::{KeyEvent, MouseEvent};
use crate::types::remote_desktop::session::{DesktopSession, Protocol, SessionInput};

// ── Global statics ──

static STORE: OnceLock<Arc<DesktopSessionStore>> = OnceLock::new();
static ACTIVE_CONNECTIONS: OnceLock<Arc<Mutex<HashMap<String, Box<dyn DesktopClient + Send>>>>> =
    OnceLock::new();
static CANCEL_TOKENS: OnceLock<Arc<Mutex<HashMap<String, bool>>>> = OnceLock::new();

fn store() -> &'static Arc<DesktopSessionStore> {
    STORE.get().expect("DesktopSessionStore not initialized")
}

fn active() -> &'static Arc<Mutex<HashMap<String, Box<dyn DesktopClient + Send>>>> {
    ACTIVE_CONNECTIONS
        .get()
        .expect("ACTIVE_CONNECTIONS not initialized")
}

fn cancel_tokens() -> &'static Arc<Mutex<HashMap<String, bool>>> {
    CANCEL_TOKENS
        .get()
        .expect("CANCEL_TOKENS not initialized")
}

// ── Init ──

pub async fn remote_desktop_init() -> Result<(), String> {
    let s = DesktopSessionStore::new().map_err(|e| format!("DesktopSessionStore init failed: {e}"))?;
    STORE.get_or_init(|| Arc::new(s));
    ACTIVE_CONNECTIONS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
    CANCEL_TOKENS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
    Ok(())
}

// ── Session CRUD ──

#[tauri::command]
pub async fn rd_list_sessions() -> Result<Vec<DesktopSession>, String> {
    store().list_sessions()
}

#[tauri::command]
pub async fn rd_create_session(
    input: SessionInput,
    password: String,
) -> Result<DesktopSession, String> {
    let session = store().create_session(input)?;
    store().set_session_password(&session.id, &password)?;
    Ok(session)
}

#[tauri::command]
pub async fn rd_update_session(id: String, input: SessionInput) -> Result<DesktopSession, String> {
    store().update_session(&id, input)
}

#[tauri::command]
pub async fn rd_delete_session(id: String) -> Result<(), String> {
    // Disconnect if active
    {
        let mut map = active().lock().await;
        if let Some(mut client) = map.remove(&id) {
            let _ = client.disconnect().await;
        }
    }
    cancel_tokens().lock().await.remove(&id);
    store().delete_session(&id)
}

// ── Connect / Disconnect ──

#[tauri::command]
pub async fn rd_connect(app: AppHandle, session_id: String, password: String) -> Result<(), String> {
    // Look up session
    let session = store()
        .get_session(&session_id)?
        .ok_or_else(|| "Session not found".to_string())?;

    // Create the appropriate client
    let mut client: Box<dyn DesktopClient + Send> = match session.protocol {
        Protocol::Vnc => Box::new(VncClient::new(session_id.clone())),
        Protocol::Rdp => Box::new(RdpClient::new(session_id.clone())),
    };

    // Connect
    client.connect(&session, &password).await?;

    // Store active connection
    active().lock().await.insert(session_id.clone(), client);

    // Set cancel token to false
    cancel_tokens()
        .lock()
        .await
        .insert(session_id.clone(), false);

    // Spawn background framebuffer polling loop
    let sid = session_id.clone();
    let app_clone = app.clone();
    tokio::spawn(async move {
        loop {
            // Check cancel token
            if cancel_tokens()
                .lock()
                .await
                .get(&sid)
                .copied()
                .unwrap_or(false)
            {
                break;
            }

            let frame_result = {
                let mut map = active().lock().await;
                match map.get_mut(&sid) {
                    Some(client) => {
                        match client.poll_frame().await {
                            Ok(f) => f,
                            Err(e) => {
                                eprintln!("[azurepath] rd poll error for {sid}: {e}");
                                break; // exit loop on error
                            }
                        }
                    }
                    None => break, // client removed
                }
            };

            if let Some(desktop_frame) = frame_result {
                let _ = app_clone.emit("rd:frame", &desktop_frame);
            }

            tokio::time::sleep(Duration::from_millis(33)).await;
        }

        // Cleanup
        let mut map = active().lock().await;
        if let Some(mut client) = map.remove(&sid) {
            let _ = client.disconnect().await;
        }
        cancel_tokens().lock().await.remove(&sid);
    });

    Ok(())
}

#[tauri::command]
pub async fn rd_disconnect(session_id: String) -> Result<(), String> {
    cancel_tokens()
        .lock()
        .await
        .insert(session_id.clone(), true);

    let mut map = active().lock().await;
    if let Some(mut client) = map.remove(&session_id) {
        client.disconnect().await?;
    }
    Ok(())
}

// ── Resize ──

#[tauri::command]
pub async fn rd_resize(session_id: String, width: u16, height: u16) -> Result<(), String> {
    let mut map = active().lock().await;
    if let Some(client) = map.get_mut(&session_id) {
        client.resize(width, height).await?;
    }
    Ok(())
}

// ── Input ──

#[tauri::command]
pub async fn rd_send_key(session_id: String, event: KeyEvent) -> Result<(), String> {
    let mut map = active().lock().await;
    if let Some(client) = map.get_mut(&session_id) {
        client.send_key_event(event).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn rd_send_mouse(session_id: String, event: MouseEvent) -> Result<(), String> {
    let mut map = active().lock().await;
    if let Some(client) = map.get_mut(&session_id) {
        client.send_mouse_event(event).await?;
    }
    Ok(())
}

// ── Clipboard ──

#[tauri::command]
pub async fn rd_push_clipboard(session_id: String, _text: String) -> Result<(), String> {
    // TODO: Push clipboard text to the remote session.
    // For RDP this requires cliprdr channel support, for VNC it's ServerCutText.
    let _ = session_id;
    Ok(())
}
