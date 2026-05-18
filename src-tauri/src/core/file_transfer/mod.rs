mod receiver;
mod sender;

use crate::types::file_transfer::FileTransfer;
pub use receiver::FileReceiver;
pub use sender::FileSender;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct FileTransferService {
    sender: Arc<FileSender>,
    receiver: Arc<FileReceiver>,
    /// Tracks known transfers (file_id -> FileTransfer)
    transfers: Arc<Mutex<HashMap<String, FileTransfer>>>,
    /// Current receiver listening port
    receiver_port: Arc<Mutex<Option<u16>>>,
}

impl FileTransferService {
    pub fn new() -> Result<Self, String> {
        let receiver = Arc::new(FileReceiver::new()?);
        let receiver_port = Arc::new(Mutex::new(None));

        // Start receiver listener
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
        })
    }

    /// Get the port the receiver is listening on (for sending to peers).
    pub async fn get_receiver_port(&self) -> Option<u16> {
        *self.receiver_port.lock().await
    }

    /// Initiate a file transfer to a peer.
    pub async fn initiate_transfer(
        &self,
        peer_id: &str,
        peer_addr: &str,
        receiver_port: u16,
        file_path: &str,
        filename: String,
        file_size: u64,
    ) -> Result<String, String> {
        let file_id = Uuid::new_v4().to_string();

        let transfer = FileTransfer {
            id: file_id.clone(),
            filename: filename.clone(),
            path: Some(file_path.to_string()),
            size: file_size,
            received: 0,
            status: "transferring".to_string(),
            peer_id: peer_id.to_string(),
            is_incoming: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.transfers.lock().await.insert(file_id.clone(), transfer);

        let path = std::path::PathBuf::from(file_path);
        let sender = self.sender.clone();
        let tid = file_id.clone();
        let transfers = self.transfers.clone();
        let peer_addr = peer_addr.to_string();

        tokio::spawn(async move {
            let result = sender
                .send_file(&peer_addr, receiver_port, &tid, &path)
                .await;

            let mut t = transfers.lock().await;
            if let Some(ft) = t.get_mut(&tid) {
                match result {
                    Ok(()) => {
                        ft.status = "completed".to_string();
                        ft.received = ft.size;
                    }
                    Err(e) => {
                        ft.status = format!("error: {}", e);
                    }
                }
            }
        });

        Ok(file_id)
    }

    /// Track an incoming file (created when file_request is accepted).
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
            status: "transferring".to_string(),
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
}
