mod receiver;
mod sender;

use crate::core::connection::ConnectionManager;
use crate::types::chat::Frame;
use crate::types::file_transfer::FileTransfer;
pub use receiver::FileReceiver;
pub use sender::FileSender;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};

pub struct FileResponseInfo {
    pub accepted: bool,
    pub data_port: u16,
}

#[derive(Clone)]
pub struct BroadcastFileState {
    pub file_path: String,
    pub filename: String,
    pub file_size: u64,
}

pub struct FileTransferService {
    sender: Arc<FileSender>,
    receiver: Arc<FileReceiver>,
    /// Tracks known transfers (file_id -> FileTransfer)
    transfers: Arc<Mutex<HashMap<String, FileTransfer>>>,
    /// Current receiver listening port
    receiver_port: Arc<Mutex<Option<u16>>>,
    /// Pending FileResponse oneshot channels (file_id -> sender)
    pending_responses: Arc<Mutex<HashMap<String, oneshot::Sender<FileResponseInfo>>>>,
    /// Map file_id -> sender_peer_id for incoming requests
    request_senders: Arc<Mutex<HashMap<String, String>>>,
    /// Connection manager for sending progress frames
    conn_mgr: Arc<Mutex<Option<Arc<ConnectionManager>>>>,
    /// Broadcast file state: broadcast_file_id -> file metadata
    broadcast_files: Arc<Mutex<HashMap<String, BroadcastFileState>>>,
}

impl FileTransferService {
    pub fn new() -> Result<Self, String> {
        let receiver = Arc::new(FileReceiver::new()?);
        let receiver_port = Arc::new(Mutex::new(None));

        let recv = receiver.clone();
        let port_holder = receiver_port.clone();
        tokio::spawn(async move {
            match recv.start_listener().await {
                Ok(port) => {
                    *port_holder.lock().await = Some(port);
                    println!("[file] Receiver ready on port {}", port);
                }
                Err(e) => eprintln!("[file] Failed to start receiver: {}", e),
            }
        });

        Ok(Self {
            sender: Arc::new(FileSender::new()),
            receiver,
            transfers: Arc::new(Mutex::new(HashMap::new())),
            receiver_port,
            pending_responses: Arc::new(Mutex::new(HashMap::new())),
            request_senders: Arc::new(Mutex::new(HashMap::new())),
            conn_mgr: Arc::new(Mutex::new(None)),
            broadcast_files: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn set_conn_mgr(&self, mgr: Arc<ConnectionManager>) {
        *self.conn_mgr.lock().await = Some(mgr);
    }

    /// Get the port the receiver is listening on.
    pub async fn get_receiver_port(&self) -> Option<u16> {
        *self.receiver_port.lock().await
    }

    /// Register a pending response channel for a file request.
    pub async fn register_pending_response(&self, file_id: &str, tx: oneshot::Sender<FileResponseInfo>) {
        self.pending_responses.lock().await.insert(file_id.to_string(), tx);
    }

    /// Deliver a FileResponse to the waiting sender. Returns true if delivered.
    pub async fn deliver_response(&self, file_id: &str, info: FileResponseInfo) -> bool {
        let mut pending = self.pending_responses.lock().await;
        if let Some(tx) = pending.remove(file_id) {
            tx.send(info).is_ok()
        } else {
            false
        }
    }

    /// Start a transfer after receiving FileResponse.accepted.
    /// Also spawns a periodic progress-reporting task.
    pub async fn start_transfer_after_response(
        &self,
        file_id: &str,
        peer_addr: &str,
        data_port: u16,
        file_path: &str,
        filename: &str,
        file_size: u64,
        peer_id: &str,
    ) {
        let transfer = FileTransfer {
            id: file_id.to_string(),
            filename: filename.to_string(),
            path: Some(file_path.to_string()),
            size: file_size,
            received: 0,
            status: "transferring".to_string(),
            peer_id: peer_id.to_string(),
            is_incoming: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.transfers.lock().await.insert(file_id.to_string(), transfer);

        let path = std::path::PathBuf::from(file_path);
        let sender = self.sender.clone();
        let tid = file_id.to_string();
        let transfers = self.transfers.clone();
        let peer_addr = peer_addr.to_string();
        let conn_mgr = self.conn_mgr.clone();
        let peer_id = peer_id.to_string();

        tokio::spawn(async move {
            // One-shot progress reporting before transfer
            let result = sender
                .send_file(&peer_addr, data_port, &tid, &path)
                .await;

            let final_status = match &result {
                Ok(()) => "completed".to_string(),
                Err(e) => format!("error: {}", e),
            };

            let mut t = transfers.lock().await;
            if let Some(ft) = t.get_mut(&tid) {
                ft.status = final_status.clone();
                if result.is_ok() {
                    ft.received = ft.size;
                }
            }
            drop(t);

            // Send file_complete / file_ack frames over chat connection
            if result.is_ok() {
                if let Some(mgr) = conn_mgr.lock().await.as_ref() {
                    if !peer_id.is_empty() {
                        let _ = mgr
                            .send(&peer_id, &Frame::FileComplete {
                                file_id: tid.clone(),
                            })
                            .await;
                    }
                }
            }
        });
    }

    /// Register an incoming file request's sender peer_id.
    pub async fn register_request_sender(&self, file_id: &str, peer_id: &str) {
        self.request_senders.lock().await.insert(file_id.to_string(), peer_id.to_string());
    }

    /// Get the sender peer_id for an incoming file request.
    pub async fn get_request_sender(&self, file_id: &str) -> Option<String> {
        self.request_senders.lock().await.get(file_id).cloned()
    }

    /// Remove a request sender mapping.
    pub async fn remove_request_sender(&self, file_id: &str) {
        self.request_senders.lock().await.remove(file_id);
    }

    /// Track an incoming file (created when file_request is received).
    pub async fn register_incoming(
        &self,
        file_id: &str,
        filename: &str,
        file_size: u64,
        peer_id: &str,
    ) {
        let transfer = FileTransfer {
            id: file_id.to_string(),
            filename: filename.to_string(),
            path: None,
            size: file_size,
            received: 0,
            status: "pending".to_string(),
            peer_id: peer_id.to_string(),
            is_incoming: true,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.transfers.lock().await.insert(file_id.to_string(), transfer);
    }

    /// Update transfer progress.
    pub async fn update_progress(&self, file_id: &str, received: u64, total: u64) {
        let mut transfers = self.transfers.lock().await;
        if let Some(ft) = transfers.get_mut(file_id) {
            ft.received = received;
            ft.size = total;
        }
    }

    /// Mark a transfer as complete.
    pub async fn mark_complete(&self, file_id: &str, path: Option<String>) {
        let mut transfers = self.transfers.lock().await;
        if let Some(ft) = transfers.get_mut(file_id) {
            ft.status = "completed".to_string();
            ft.path = path;
            ft.received = ft.size;
        }
    }

    /// Mark a transfer as errored.
    pub async fn mark_error(&self, file_id: &str, error: &str) {
        let mut transfers = self.transfers.lock().await;
        if let Some(ft) = transfers.get_mut(file_id) {
            ft.status = format!("error: {}", error);
        }
    }

    /// List all known transfers.
    pub async fn list_transfers(&self) -> Vec<FileTransfer> {
        let transfers = self.transfers.lock().await;
        let mut list: Vec<FileTransfer> = transfers.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }

    /// Register a broadcast file (sent to all peers).
    pub async fn register_broadcast(&self, file_id: &str, file_path: &str, filename: &str, file_size: u64) {
        self.broadcast_files.lock().await.insert(file_id.to_string(), BroadcastFileState {
            file_path: file_path.to_string(),
            filename: filename.to_string(),
            file_size,
        });
    }

    /// Check if a file_id is a broadcast file.
    pub async fn is_broadcast_file(&self, file_id: &str) -> bool {
        self.broadcast_files.lock().await.contains_key(file_id)
    }

    /// Get broadcast file state by file_id.
    pub async fn get_broadcast_info(&self, file_id: &str) -> Option<BroadcastFileState> {
        self.broadcast_files.lock().await.get(file_id).cloned()
    }
}
