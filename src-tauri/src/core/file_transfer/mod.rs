pub mod receiver;
mod sender;

use crate::core::connection::ConnectionManager;
use crate::core::utils::emit_or_warn;
use crate::types::chat::Frame;
use crate::types::file_transfer::FileTransfer;
pub use receiver::FileReceiver;
pub use sender::FileSender;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use tracing::info;

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
    #[allow(dead_code)]
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
    /// Create a new file transfer service and start the receiver synchronously.
    /// The returned service always has a valid receiver port.
    pub async fn new() -> Result<Self, String> {
        let receiver = Arc::new(FileReceiver::new()?);

        // Start the receiver inline (blocks until the TCP listener is bound),
        // so the port is immediately available to callers like file_accept.
        let port = receiver.clone().start_listener().await?;
        info!("[file] Receiver ready on port {}", port);

        Ok(Self {
            sender: Arc::new(FileSender::new()),
            receiver,
            transfers: Arc::new(Mutex::new(HashMap::new())),
            receiver_port: Arc::new(Mutex::new(Some(port))),
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

    /// Remove a pending response entry (e.g. on timeout or error).
    /// Prevents memory leaks when the response never arrives.
    pub async fn remove_pending_response(&self, file_id: &str) {
        self.pending_responses.lock().await.remove(file_id);
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
            download_url: None,
        };
        self.transfers.lock().await.insert(file_id.to_string(), transfer);

        let path = std::path::PathBuf::from(file_path);
        let sender = self.sender.clone();
        let tid = file_id.to_string();
        let transfers = self.transfers.clone();
        let peer_addr = peer_addr.to_string();
        let conn_mgr = self.conn_mgr.clone();
        let peer_id = peer_id.to_string();
        let filename_owned = filename.to_string();

        tokio::spawn(async move {
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

            if result.is_ok() {
                // Register with file server for download URL
                let path_str = path.to_string_lossy().to_string();
                if let Some(srv) = crate::commands::file_transfer::file_server_handle() {
                    srv.register_file(&tid, &path_str);
                    let download_url = srv.download_url(&tid, &filename_owned);

                    // Set download URL on transfer
                    let mut t2 = transfers.lock().await;
                    if let Some(ft) = t2.get_mut(&tid) {
                        ft.download_url = Some(download_url.clone());
                    }
                    drop(t2);

                    // Emit file:complete event to frontend
                    if let Some(app) = crate::commands::file_transfer::app_handle() {
                        emit_or_warn(app, "file:complete", &serde_json::json!({
                            "fileId": tid,
                            "path": path_str,
                            "downloadUrl": download_url,
                        }));
                    }
                }

                // Send file_complete frame over chat connection
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
    #[allow(dead_code)]
    pub async fn get_request_sender(&self, file_id: &str) -> Option<String> {
        self.request_senders.lock().await.get(file_id).cloned()
    }

    /// Remove a request sender mapping.
    #[allow(dead_code)]
    pub async fn remove_request_sender(&self, file_id: &str) {
        self.request_senders.lock().await.remove(file_id);
    }

    /// Atomically take and remove the sender peer_id for an incoming file request.
    /// Prevents race conditions where two handlers get the same sender.
    pub async fn take_request_sender(&self, file_id: &str) -> Option<String> {
        self.request_senders.lock().await.remove(file_id)
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
            download_url: None,
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
    #[cfg_attr(not(test), allow(dead_code))]
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

    /// Get filename for a transfer.
    pub async fn get_filename(&self, file_id: &str) -> Option<String> {
        let transfers = self.transfers.lock().await;
        transfers.get(file_id).map(|t| t.filename.clone())
    }

    /// Set download URL on a transfer.
    pub async fn set_download_url(&self, file_id: &str, url: &str) {
        let mut transfers = self.transfers.lock().await;
        if let Some(ft) = transfers.get_mut(file_id) {
            ft.download_url = Some(url.to_string());
        }
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

    /// Remove a broadcast file record after it has been processed.
    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn remove_broadcast(&self, file_id: &str) {
        self.broadcast_files.lock().await.remove(file_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_list_transfers() {
        let svc = FileTransferService::new().await.unwrap();

        svc.register_incoming("id1", "test.txt", 1024, "peer1").await;
        svc.register_incoming("id2", "photo.jpg", 2048, "peer2").await;

        let list = svc.list_transfers().await;
        assert_eq!(list.len(), 2);

        let id1 = list.iter().find(|t| t.id == "id1").unwrap();
        assert_eq!(id1.filename, "test.txt");
        assert_eq!(id1.size, 1024);
        assert_eq!(id1.status, "pending");
        assert!(id1.is_incoming);

        let id2 = list.iter().find(|t| t.id == "id2").unwrap();
        assert_eq!(id2.filename, "photo.jpg");
        assert_eq!(id2.size, 2048);
    }

    #[tokio::test]
    async fn test_update_progress() {
        let svc = FileTransferService::new().await.unwrap();
        svc.register_incoming("id1", "test.txt", 1000, "peer1").await;

        svc.update_progress("id1", 500, 1000).await;
        let list = svc.list_transfers().await;
        let t = list.iter().find(|t| t.id == "id1").unwrap();
        assert_eq!(t.received, 500);
        assert_eq!(t.size, 1000);
    }

    #[tokio::test]
    async fn test_mark_complete() {
        let svc = FileTransferService::new().await.unwrap();
        svc.register_incoming("id1", "test.txt", 1000, "peer1").await;

        svc.mark_complete("id1", Some("/tmp/test.txt".to_string())).await;
        let list = svc.list_transfers().await;
        let t = list.iter().find(|t| t.id == "id1").unwrap();
        assert_eq!(t.status, "completed");
        assert_eq!(t.path.as_deref(), Some("/tmp/test.txt"));
        assert_eq!(t.received, t.size);
    }

    #[tokio::test]
    async fn test_mark_error() {
        let svc = FileTransferService::new().await.unwrap();
        svc.register_incoming("id1", "test.txt", 1000, "peer1").await;

        svc.mark_error("id1", "disk full").await;
        let list = svc.list_transfers().await;
        let t = list.iter().find(|t| t.id == "id1").unwrap();
        assert!(t.status.starts_with("error:"));
    }

    #[tokio::test]
    async fn test_request_sender_take_is_atomic() {
        let svc = FileTransferService::new().await.unwrap();
        svc.register_request_sender("id1", "peer1").await;

        // First take should succeed
        let taken = svc.take_request_sender("id1").await;
        assert_eq!(taken, Some("peer1".to_string()));

        // Second take should return None (already removed)
        let taken2 = svc.take_request_sender("id1").await;
        assert_eq!(taken2, None);
    }

    #[tokio::test]
    async fn test_broadcast_register_and_cleanup() {
        let svc = FileTransferService::new().await.unwrap();
        svc.register_broadcast("id1", "/tmp/test.txt", "test.txt", 1000).await;

        assert!(svc.is_broadcast_file("id1").await);
        let info = svc.get_broadcast_info("id1").await;
        assert!(info.is_some());
        assert_eq!(info.unwrap().filename, "test.txt");

        svc.remove_broadcast("id1").await;
        assert!(!svc.is_broadcast_file("id1").await);
    }

    #[tokio::test]
    async fn test_get_filename() {
        let svc = FileTransferService::new().await.unwrap();
        svc.register_incoming("id1", "test.txt", 1000, "peer1").await;

        let name = svc.get_filename("id1").await;
        assert_eq!(name, Some("test.txt".to_string()));

        let missing = svc.get_filename("nonexistent").await;
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_set_download_url() {
        let svc = FileTransferService::new().await.unwrap();
        svc.register_incoming("id1", "test.txt", 1000, "peer1").await;

        svc.set_download_url("id1", "http://localhost:8080/download/id1/test.txt").await;
        let list = svc.list_transfers().await;
        let t = list.iter().find(|t| t.id == "id1").unwrap();
        assert_eq!(
            t.download_url.as_deref(),
            Some("http://localhost:8080/download/id1/test.txt")
        );
    }

    #[tokio::test]
    async fn test_deliver_response() {
        let svc = FileTransferService::new().await.unwrap();
        let (tx, rx) = oneshot::channel::<FileResponseInfo>();
        svc.register_pending_response("id1", tx).await;

        let delivered = svc.deliver_response("id1", FileResponseInfo { accepted: true, data_port: 12345 }).await;
        assert!(delivered);

        let response = rx.await.unwrap();
        assert!(response.accepted);
        assert_eq!(response.data_port, 12345);

        // Second delivery should fail (already consumed)
        let delivered2 = svc.deliver_response("id1", FileResponseInfo { accepted: false, data_port: 0 }).await;
        assert!(!delivered2);
    }

    #[tokio::test]
    async fn test_receiver_port_always_ready() {
        let svc = FileTransferService::new().await.unwrap();
        // Port is always available now (receiver started synchronously in new())
        let port = svc.get_receiver_port().await;
        assert!(port.is_some(), "receiver port should be available immediately after construction");
        assert!(port.unwrap() > 0, "receiver port should be a valid port number");
    }
}
