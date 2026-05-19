use crate::core::connection::IncomingFrame;
use crate::core::file_transfer::{FileResponseInfo, FileTransferService};
use crate::types::file_transfer::FileTransfer;
use std::sync::OnceLock;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;
use uuid::Uuid;

static FILE_SVC: OnceLock<Arc<FileTransferService>> = OnceLock::new();

/// Handle incoming file-related frames from the connection module.
pub(crate) async fn handle_frame(incoming: &IncomingFrame, app: &AppHandle) {
    let svc = match FILE_SVC.get() {
        Some(s) => s,
        None => return,
    };

    match &incoming.frame {
        crate::types::chat::Frame::FileRequest {
            file_id,
            filename,
            size,
            from: _,
        } => {
            // Remember which peer sent this request (for routing file_accept back)
            svc.register_request_sender(file_id, &incoming.peer_id).await;
            svc.register_incoming(file_id, filename, *size, &incoming.peer_id).await;
            let _ = app.emit(
                "file:request",
                serde_json::json!({
                    "fileId": file_id,
                    "filename": filename,
                    "size": size,
                    "from": incoming.peer_id,
                }),
            );
        }
        crate::types::chat::Frame::FileResponse {
            file_id,
            accepted,
            data_port,
        } => {
            let accepted = *accepted;
            let data_port = *data_port;

            // Check if this is a broadcast file (multiple peers may respond)
            if svc.is_broadcast_file(file_id).await {
                if accepted {
                    if let Some(info) = svc.get_broadcast_info(file_id).await {
                        let transfer_id = Uuid::new_v4().to_string();
                        // Look up responding peer's IP address
                        let peer_addr = match crate::commands::discovery::DISCOVERY.get() {
                            Some(d) => d.get_peer(&incoming.peer_id).await
                                .map(|p| p.ip.clone())
                                .unwrap_or_default(),
                            None => "unknown".to_string(),
                        };
                        svc.start_transfer_after_response(
                            &transfer_id,
                            &peer_addr,
                            data_port,
                            &info.file_path,
                            &info.filename,
                            info.file_size,
                            &incoming.peer_id,
                        ).await;
                    }
                }
            } else {
                // Original oneshot flow for unicast
                let _ = svc.deliver_response(file_id, FileResponseInfo { accepted, data_port }).await;
            }
        }
        crate::types::chat::Frame::FileProgress {
            file_id,
            received,
            total,
            speed,
        } => {
            svc.update_progress(file_id, *received, *total).await;
            let _ = app.emit(
                "file:progress",
                serde_json::json!({
                    "fileId": file_id,
                    "received": received,
                    "total": total,
                    "speed": speed,
                }),
            );
        }
        crate::types::chat::Frame::FileComplete { file_id } => {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_default();
            let path = std::path::PathBuf::from(&home)
                .join("AzurePath/downloads")
                .join("received");
            svc.mark_complete(file_id, path.to_str().map(|s| s.to_string()))
                .await;
            let _ = app.emit(
                "file:complete",
                serde_json::json!({
                    "fileId": file_id,
                    "path": path.to_string_lossy(),
                }),
            );
        }
        crate::types::chat::Frame::FileAck { file_id } => {
            let _ = app.emit(
                "file:ack",
                serde_json::json!({
                    "fileId": file_id,
                }),
            );
        }
        _ => {}
    }
}

#[tauri::command]
pub async fn file_transfer_init(_app: AppHandle) -> Result<(), String> {
    if FILE_SVC.get().is_some() {
        return Ok(());
    }

    let svc = Arc::new(FileTransferService::new()?);
    FILE_SVC
        .set(svc)
        .map_err(|_| "Already initialized".to_string())?;

    Ok(())
}

pub(crate) async fn set_file_conn_mgr(mgr: Arc<crate::core::connection::ConnectionManager>) {
    if let Some(svc) = FILE_SVC.get() {
        svc.set_conn_mgr(mgr).await;
    }
}

#[tauri::command]
pub async fn file_send(
    target: String,
    path: String,
) -> Result<String, String> {
    let svc = FILE_SVC.get().ok_or("File transfer not initialized")?;

    // Get file metadata
    let file_path = std::path::PathBuf::from(&path);
    let metadata =
        std::fs::metadata(&file_path).map_err(|e| format!("Cannot read file: {}", e))?;
    let file_size = metadata.len();
    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?
        .to_string();

    // Get peer info
    let peer = match crate::commands::discovery::DISCOVERY.get() {
        Some(d) => d.get_peer(&target).await,
        None => None,
    };
    let peer_addr = peer
        .as_ref()
        .map(|p| p.ip.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let conn_mgr = match crate::commands::chat::CONN_MGR.get() {
        Some(c) => c,
        None => return Err("Connection manager not initialized".to_string()),
    };

    // Generate real file_id
    let file_id = Uuid::new_v4().to_string();

    // Create oneshot channel for the response
    let (tx, rx) = oneshot::channel::<FileResponseInfo>();
    svc.register_pending_response(&file_id, tx).await;

    // Send FileRequest to the specific peer
    conn_mgr
        .send(
            &target,
            &crate::types::chat::Frame::FileRequest {
                file_id: file_id.clone(),
                filename: filename.clone(),
                size: file_size,
                from: crate::core::discovery::my_id().await,
            },
        )
        .await?;

    // Wait for FileResponse with timeout
    let response = tokio::time::timeout(
        tokio::time::Duration::from_secs(30),
        rx,
    )
    .await
    .map_err(|_| "File request timed out (no response in 30s)".to_string())?
    .map_err(|_| "File response channel closed".to_string())?;

    if !response.accepted {
        return Err("File transfer rejected by peer".to_string());
    }

    // Start the actual data transfer
    svc.start_transfer_after_response(
        &file_id,
        &peer_addr,
        response.data_port,
        &path,
        &filename,
        file_size,
        &target,
    )
    .await;

    Ok(file_id)
}

#[tauri::command]
pub async fn file_accept(file_id: String, receiver_port: u16) -> Result<(), String> {
    let conn_mgr = match crate::commands::chat::CONN_MGR.get() {
        Some(c) => c,
        None => return Err("Connection manager not initialized".to_string()),
    };

    // Look up which peer sent this request
    let svc = FILE_SVC.get().ok_or("File transfer not initialized")?;
    let sender_peer = svc
        .get_request_sender(&file_id)
        .await
        .ok_or("Unknown file request")?;

    svc.remove_request_sender(&file_id).await;

    // Send FileResponse only to the requesting peer
    conn_mgr
        .send(
            &sender_peer,
            &crate::types::chat::Frame::FileResponse {
                file_id,
                accepted: true,
                data_port: receiver_port,
            },
        )
        .await?;

    Ok(())
}

#[tauri::command]
pub async fn file_reject(file_id: String) -> Result<(), String> {
    let conn_mgr = match crate::commands::chat::CONN_MGR.get() {
        Some(c) => c,
        None => return Err("Connection manager not initialized".to_string()),
    };

    let svc = FILE_SVC.get().ok_or("File transfer not initialized")?;
    let sender_peer = svc
        .get_request_sender(&file_id)
        .await
        .ok_or("Unknown file request")?;

    svc.remove_request_sender(&file_id).await;

    conn_mgr
        .send(
            &sender_peer,
            &crate::types::chat::Frame::FileResponse {
                file_id,
                accepted: false,
                data_port: 0,
            },
        )
        .await?;

    Ok(())
}

#[tauri::command]
pub async fn file_list() -> Result<Vec<FileTransfer>, String> {
    let svc = FILE_SVC.get().ok_or("File transfer not initialized")?;
    Ok(svc.list_transfers().await)
}

#[tauri::command]
pub async fn file_broadcast(path: String) -> Result<String, String> {
    let svc = FILE_SVC.get().ok_or("File transfer not initialized")?;

    let file_path = std::path::PathBuf::from(&path);
    let metadata =
        std::fs::metadata(&file_path).map_err(|e| format!("Cannot read file: {}", e))?;
    let file_size = metadata.len();
    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?
        .to_string();

    let conn_mgr = match crate::commands::chat::CONN_MGR.get() {
        Some(c) => c,
        None => return Err("Connection manager not initialized".to_string()),
    };

    let file_id = Uuid::new_v4().to_string();

    // Register as broadcast file so responses are handled correctly
    svc.register_broadcast(&file_id, &path, &filename, file_size).await;

    // Send FileRequest to ALL connected peers
    conn_mgr
        .broadcast(&crate::types::chat::Frame::FileRequest {
            file_id: file_id.clone(),
            filename: filename.clone(),
            size: file_size,
            from: crate::core::discovery::my_id().await,
        })
        .await;

    Ok(file_id)
}
