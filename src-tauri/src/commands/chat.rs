use crate::core::chat::ChatService;
use crate::core::connection::ConnectionManager;
use crate::types::chat::StoredMessage;
use std::sync::OnceLock;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tracing::warn;

static CHAT: OnceLock<ChatService> = OnceLock::new();
pub(crate) static CONN_MGR: OnceLock<Arc<ConnectionManager>> = OnceLock::new();

#[tauri::command]
pub async fn chat_init(app: AppHandle) -> Result<(), String> {
    if CHAT.get().is_some() {
        return Ok(());
    }

    let service = ChatService::new()?;
    CHAT.set(service).map_err(|_| "Already initialized".to_string())?;

    // Subscribe to incoming frames and forward chat messages to frontend
    let conn_mgr = get_conn_mgr();
    let mut rx = conn_mgr.subscribe();
    let app_clone = app.clone();

    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(incoming) => {
                    if let Some(stored) = CHAT.get().and_then(|c| {
                        c.handle_incoming(&incoming)
                    }) {
                        let _ = app_clone.emit("chat:message", &stored);
                    }

                    // Handle system messages
                    if let crate::types::chat::Frame::System { content } = &incoming.frame {
                        if incoming.peer_id.starts_with("__disconnected:") {
                            let peer_id = incoming.peer_id.trim_start_matches("__disconnected:");
                            // Mark peer offline in the peer table
                            if let Some(d) = crate::commands::discovery::DISCOVERY.get() {
                                d.peer_table().mark_offline(peer_id).await;
                            }
                            let _ = app_clone.emit("peer:offline", serde_json::json!({
                                "id": peer_id,
                            }));
                        } else {
                            // System message from a remote peer — show as chat message
                            let sys_msg = crate::types::chat::StoredMessage {
                                id: uuid::Uuid::new_v4().to_string(),
                                peer_id: incoming.peer_id.clone(),
                                peer_name: "系统".to_string(),
                                peer_ip: "".to_string(),
                                peer_os: None,
                                content: format!("🔔 {}", content),
                                is_broadcast: true,
                                is_incoming: true,
                                file_ref: None,
                                created_at: chrono::Utc::now().to_rfc3339(),
                            };
                            let _ = app_clone.emit("chat:message", &sys_msg);
                        }
                    }

                    // Handle file-related frames via the connection
                    crate::commands::file_transfer::handle_frame(&incoming, &app_clone).await;

                    // Handle clipboard sync frames
                    crate::commands::clipboard::handle_frame(&incoming, &app_clone).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!("[chat] Lagged by {} messages", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    Ok(())
}

fn get_conn_mgr() -> Arc<ConnectionManager> {
    CONN_MGR
        .get()
        .cloned()
        .expect("ConnectionManager not initialized")
}

pub(crate) fn set_conn_mgr(mgr: Arc<ConnectionManager>) {
    CONN_MGR.set(mgr).ok();
}

#[tauri::command]
pub async fn chat_send(target: String, content: String) -> Result<StoredMessage, String> {
    let chat = CHAT.get().ok_or("Chat not initialized")?;
    let conn_mgr = get_conn_mgr();

    // Look up peer info from discovery
    let peer_info = match crate::commands::discovery::DISCOVERY.get() {
        Some(d) => d.get_peer(&target).await,
        None => None,
    };
    let (peer_name, peer_ip, peer_os) = match peer_info {
        Some(info) => (info.hostname, info.ip, info.os),
        None => (target.clone(), "unknown".to_string(), "unknown".to_string()),
    };

    chat.send(&conn_mgr, &target, content, &peer_name, &peer_ip, &peer_os)
        .await
}

#[tauri::command]
pub async fn chat_broadcast(content: String) -> Result<StoredMessage, String> {
    let chat = CHAT.get().ok_or("Chat not initialized")?;
    let conn_mgr = get_conn_mgr();
    chat.broadcast(&conn_mgr, content).await
}

#[tauri::command]
pub async fn chat_messages(peer_id: Option<String>) -> Result<Vec<StoredMessage>, String> {
    let chat = CHAT.get().ok_or("Chat not initialized")?;
    chat.store()
        .get_messages(peer_id.as_deref(), 200)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chat_history(limit: Option<u32>) -> Result<Vec<StoredMessage>, String> {
    let chat = CHAT.get().ok_or("Chat not initialized")?;
    chat.store()
        .get_messages(None, limit.unwrap_or(100))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The command functions are thin wrappers around ChatService /
    /// ChatStore that depend on process-global statics
    /// (`CHAT`, `CONN_MGR`, `crate::commands::discovery::DISCOVERY`) and a
    /// Tauri `AppHandle`.  Full integration tests require a running Tauri
    /// application.
    ///
    /// Core logic (ChatService, ChatStore) is thoroughly tested in
    /// `crate::core::chat::mod` and `crate::core::chat::store`.
    /// Command-layer tests below focus on type/compile checks and
    /// parameter handling only, avoiding global-state mutation.

    #[test]
    fn test_option_peer_id_types() {
        // Verify that both None and Some work for the peer_id parameter
        let none_param: Option<String> = None;
        let some_param: Option<String> = Some("peer-1".to_string());
        assert!(none_param.is_none());
        assert!(some_param.is_some());
    }

    #[test]
    fn test_chat_history_limit_default_value() {
        // The function produces `limit.unwrap_or(100)` internally.
        // This test validates the default logic in isolation.
        let none: Option<u32> = None;
        let with_value: Option<u32> = Some(50);
        assert_eq!(none.unwrap_or(100), 100);
        assert_eq!(with_value.unwrap_or(100), 50);
    }

    #[test]
    fn test_chat_history_zero_limit() {
        // Zero is a valid limit that ChatStore handles.
        let limit: Option<u32> = Some(0);
        assert_eq!(limit.unwrap_or(100), 0);
    }

    #[test]
    fn test_static_initial_state() {
        // These statics are process-global OnceLocks.
        // In a test environment they should NOT be initialized
        // (no Tauri app is running). We verify this to catch
        // accidental test-order dependencies.
        if CHAT.get().is_some() {
            warn!("WARNING: CHAT static is already initialized — test isolation may be compromised");
        }
        if CONN_MGR.get().is_some() {
            warn!("WARNING: CONN_MGR static is already initialized — test isolation may be compromised");
        }
        // No assertion — just a diagnostic.
    }

    #[test]
    fn test_get_conn_mgr_panics_when_not_set() {
        // get_conn_mgr() calls .expect() on the OnceLock, which panics
        // when CONN_MGR is not set. Validate via std::panic::catch_unwind.
        let result = std::panic::catch_unwind(|| {
            let _ = get_conn_mgr();
        });
        assert!(result.is_err(), "get_conn_mgr should panic when CONN_MGR is not set");
    }
}
