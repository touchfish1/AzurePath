//! WebSocket session management.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::core::utils::emit_or_warn;

pub struct WsSession {
    pub url: String,
    pub connected: Arc<AtomicBool>,
    tx: mpsc::Sender<String>,
}

impl WsSession {
    pub async fn connect(url: &str, app: AppHandle) -> Result<Self, String> {
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| format!("WebSocket connection failed: {e}"))?;

        let (mut write, mut read) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel::<String>(256);
        let connected = Arc::new(AtomicBool::new(true));
        let connected_clone = connected.clone();
        let app_clone = app.clone();
        let url_owned = url.to_string();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = rx.recv() => {
                        if write.send(Message::Text(msg)).await.is_err() {
                            break;
                        }
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                emit_or_warn(&app, "ws:message", &serde_json::json!({
                                    "direction": "received",
                                    "content": text,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                }));
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            Some(Err(e)) => {
                                emit_or_warn(&app, "ws:error", &serde_json::json!({
                                    "error": format!("{e}")
                                }));
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            connected_clone.store(false, Ordering::Relaxed);
            emit_or_warn(&app, "ws:disconnected", &serde_json::json!({
                "code": 1000, "reason": "Connection closed"
            }));
        });

        emit_or_warn(&app_clone, "ws:connected", &serde_json::json!({
            "url": url_owned
        }));

        Ok(Self {
            url: url.to_string(),
            connected,
            tx,
        })
    }

    pub async fn send(&self, text: String) -> Result<(), String> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err("WebSocket not connected".into());
        }
        self.tx
            .send(text)
            .await
            .map_err(|_| "WebSocket send channel closed".to_string())
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}
