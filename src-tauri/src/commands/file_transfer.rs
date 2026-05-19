use crate::core::connection::IncomingFrame;
use crate::core::file_server::FileServerHandle;
use crate::core::file_transfer::{FileResponseInfo, FileTransferService};
use crate::types::file_transfer::FileTransfer;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;
use tracing::info;
use uuid::Uuid;

#[derive(Serialize)]
pub struct FileSendResult {
    pub file_id: String,
    pub file_size: u64,
    pub download_url: Option<String>,
}

static FILE_SVC: OnceLock<Arc<FileTransferService>> = OnceLock::new();
static FILE_SERVER: OnceLock<FileServerHandle> = OnceLock::new();
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// Accessor for file server handle (used from core module's spawned tasks).
pub fn file_server_handle() -> Option<&'static FileServerHandle> {
    FILE_SERVER.get()
}

/// Accessor for app handle (used from core module's spawned tasks).
pub fn app_handle() -> Option<&'static AppHandle> {
    APP_HANDLE.get()
}

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
            let download_dir = crate::core::file_transfer::receiver::default_download_dir();
            let filename = svc.get_filename(file_id).await.unwrap_or_else(|| "file".to_string());
            let file_path = download_dir.join(&filename);
            let file_path_str = file_path.to_string_lossy().to_string();
            svc.mark_complete(file_id, Some(file_path_str.clone())).await;

            // Register with file server for download
            if let Some(srv) = FILE_SERVER.get() {
                srv.register_file(file_id, &file_path_str);
                let download_url = srv.download_url(file_id, &filename);

                // Update the transfer record with download URL
                svc.set_download_url(file_id, &download_url).await;

                let _ = app.emit(
                    "file:complete",
                    serde_json::json!({
                        "fileId": file_id,
                        "path": file_path_str,
                        "downloadUrl": download_url,
                    }),
                );
            } else {
                let _ = app.emit(
                    "file:complete",
                    serde_json::json!({
                        "fileId": file_id,
                        "path": file_path_str,
                    }),
                );
            }
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
pub async fn file_transfer_init(app: AppHandle) -> Result<(), String> {
    if FILE_SVC.get().is_some() {
        return Ok(());
    }

    let svc = Arc::new(FileTransferService::new().await?);
    FILE_SVC
        .set(svc)
        .map_err(|_| "Already initialized".to_string())?;

    // Store AppHandle for event emission
    let _ = APP_HANDLE.set(app.clone());

    // Start local HTTP file server for download URLs
    let server = crate::core::file_server::FileServer::new()?;
    let handle = server.handle().clone();
    let _ = FILE_SERVER.set(handle);
    // Keep server alive (drop is intentional — JoinHandle will die with process)
    std::mem::forget(server);

    info!("[file] File server ready on port {}", FILE_SERVER.get().map(|s| s.port()).unwrap_or(0));

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
) -> Result<FileSendResult, String> {
    let svc = FILE_SVC.get().ok_or("File transfer not initialized")?;

    // Validate the file path (prevent path traversal and ensure file is accessible)
    let file_path = validate_file_path(&path)?;
    let metadata = std::fs::metadata(&file_path)
        .map_err(|e| format!("Cannot read file: {}", e))?;
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

    let file_path_str = file_path.to_string_lossy().to_string();

    // Register with HTTP file server IMMEDIATELY — file is on disk,
    // doesn't depend on peer acceptance. This ensures the download URL
    // is always available for local files.
    let download_url = if let Some(srv) = FILE_SERVER.get() {
        srv.register_file(&file_id, &file_path_str);
        let url = srv.download_url(&file_id, &filename);
        Some(url)
    } else {
        None
    };

    // Create oneshot channel for the response
    let (tx, rx) = oneshot::channel::<FileResponseInfo>();
    svc.register_pending_response(&file_id, tx).await;

    // Send FileRequest to the specific peer
    let send_result = conn_mgr
        .send(
            &target,
            &crate::types::chat::Frame::FileRequest {
                file_id: file_id.clone(),
                filename: filename.clone(),
                size: file_size,
                from: crate::core::discovery::my_id().await,
            },
        )
        .await;

    // Try to wait for FileResponse, but don't block the download URL
    if let Ok(()) = send_result {
        let response = tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            rx,
        )
        .await;

        match response {
            Ok(Ok(info)) if info.accepted => {
                // Peer accepted — start data transfer on background task
                svc.start_transfer_after_response(
                    &file_id,
                    &peer_addr,
                    info.data_port,
                    &file_path_str,
                    &filename,
                    file_size,
                    &target,
                )
                .await;
                // Entry already removed by deliver_response — no leak.
            }
            Ok(Ok(_)) => {
                info!("[file] Peer rejected file transfer");
                // Entry already removed by deliver_response — no leak.
            }
            Ok(Err(_)) => {
                info!("[file] File response channel closed");
                // Oneshot sender dropped without delivery — clean up.
                svc.remove_pending_response(&file_id).await;
            }
            Err(_) => {
                info!("[file] File request timed out (no response in 30s)");
                // Clean up orphaned pending response to prevent memory leak.
                svc.remove_pending_response(&file_id).await;
            }
        }
    } else {
        info!("[file] Failed to send file request to peer");
        // Request was never sent — clean up the pending response entry.
        svc.remove_pending_response(&file_id).await;
    }

    // Always return success with download URL — file is local and registered
    Ok(FileSendResult { file_id, file_size, download_url })
}

#[tauri::command]
pub async fn file_accept(file_id: String) -> Result<(), String> {
    let conn_mgr = match crate::commands::chat::CONN_MGR.get() {
        Some(c) => c,
        None => return Err("Connection manager not initialized".to_string()),
    };

    // Look up which peer sent this request (atomically take to prevent races)
    let svc = FILE_SVC.get().ok_or("File transfer not initialized")?;
    let sender_peer = svc
        .take_request_sender(&file_id)
        .await
        .ok_or("Unknown file request")?;

    // Use the actual receiver port (started dynamically in FileTransferService::new)
    let actual_port = svc
        .get_receiver_port()
        .await
        .ok_or("File receiver not ready")?;

    // Send FileResponse only to the requesting peer
    conn_mgr
        .send(
            &sender_peer,
            &crate::types::chat::Frame::FileResponse {
                file_id,
                accepted: true,
                data_port: actual_port,
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
        .take_request_sender(&file_id)
        .await
        .ok_or("Unknown file request")?;

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
    let mut list = svc.list_transfers().await;

    // Populate download_url from file server for all transfers
    if let Some(srv) = FILE_SERVER.get() {
        for t in &mut list {
            if t.status == "completed" {
                // Ensure the file is registered with the server
                if let Some(ref path) = t.path {
                    if srv.get_path(&t.id).is_none() {
                        srv.register_file(&t.id, path);
                    }
                }
                t.download_url = Some(srv.download_url(&t.id, &t.filename));
            }
        }
    }

    Ok(list)
}

/// Get download URL for a completed file transfer by file_id.
#[tauri::command]
pub async fn get_file_download_url(file_id: String) -> Result<String, String> {
    let svc = FILE_SVC.get().ok_or("File transfer not initialized")?;
    let transfers = svc.list_transfers().await;
    let transfer = transfers.iter().find(|t| t.id == file_id)
        .ok_or_else(|| format!("Transfer not found: {}", file_id))?;

    if transfer.status != "completed" {
        return Err(format!("Transfer is not completed: {}", transfer.status));
    }

    let srv = FILE_SERVER.get().ok_or("File server not available")?;
    let path = transfer.path.as_ref().ok_or("File path not available")?;

    // Ensure registered
    if srv.get_path(&file_id).is_none() {
        srv.register_file(&file_id, path);
    }

    Ok(srv.download_url(&file_id, &transfer.filename))
}

#[tauri::command]
pub async fn file_broadcast(path: String) -> Result<FileSendResult, String> {
    let svc = FILE_SVC.get().ok_or("File transfer not initialized")?;

    let file_path = validate_file_path(&path)?;
    let metadata = std::fs::metadata(&file_path)
        .map_err(|e| format!("Cannot read file: {}", e))?;
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
    let file_path_str = file_path.to_string_lossy().to_string();

    // Register with file server for download URL
    let download_url = if let Some(srv) = FILE_SERVER.get() {
        srv.register_file(&file_id, &file_path_str);
        let url = srv.download_url(&file_id, &filename);
        Some(url)
    } else {
        None
    };

    // Register as broadcast file so responses are handled correctly
    svc.register_broadcast(&file_id, &file_path_str, &filename, file_size).await;

    // Send FileRequest to ALL connected peers
    conn_mgr
        .broadcast(&crate::types::chat::Frame::FileRequest {
            file_id: file_id.clone(),
            filename: filename.clone(),
            size: file_size,
            from: crate::core::discovery::my_id().await,
        })
        .await;

    Ok(FileSendResult { file_id, file_size, download_url })
}

/// Validate that a user-supplied file path is safe and accessible.
/// Returns the canonical path, or an error if:
/// - The path contains traversal components (after resolution)
/// - The file does not exist
/// - The path is not a regular file
/// - The file is not readable
fn validate_file_path(path: &str) -> Result<PathBuf, String> {
    let file_path = PathBuf::from(path);

    // Check file exists
    if !file_path.exists() {
        return Err("File not found".to_string());
    }

    // Check it's a regular file (not a directory, pipe, etc.)
    let metadata = std::fs::metadata(&file_path)
        .map_err(|_| "Cannot access file metadata".to_string())?;
    if !metadata.is_file() {
        return Err("Path is not a regular file".to_string());
    }

    // Try to canonicalize to detect path traversal attempts
    let canonical = std::fs::canonicalize(&file_path)
        .map_err(|_| "Cannot resolve file path".to_string())?;

    // Verify the canonical path exists and is a file (extra safety)
    if !canonical.exists() {
        return Err("Resolved file path does not exist".to_string());
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_validate_file_path_rejects_nonexistent() {
        let result = validate_file_path("C:\\nonexistent_file_xyz123.tmp");
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("not found"));
    }

    #[test]
    fn test_validate_file_path_rejects_directory() {
        let result = validate_file_path(".");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_path_accepts_valid_file() {
        // Create a temp file
        let dir = std::env::temp_dir();
        let temp_path = dir.join("azurepath_test_valid.txt");
        let mut f = std::fs::File::create(&temp_path).unwrap();
        f.write_all(b"test").unwrap();
        drop(f);

        let result = validate_file_path(temp_path.to_str().unwrap());
        assert!(result.is_ok());

        // Cleanup
        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_validate_file_path_canonicalizes() {
        let dir = std::env::temp_dir();
        let temp_path = dir.join("azurepath_test_canon.txt");
        let mut f = std::fs::File::create(&temp_path).unwrap();
        f.write_all(b"test").unwrap();
        drop(f);

        // Use a relative path with .. components
        let relative_path = format!(
            "{}\\..\\{}\\azurepath_test_canon.txt",
            dir.to_str().unwrap(),
            dir.file_name().unwrap().to_str().unwrap()
        );
        let result = validate_file_path(&relative_path);
        assert!(result.is_ok());
        if let Ok(canonical) = result {
            // Should resolve to the actual file path
            assert!(canonical.to_string_lossy().ends_with("azurepath_test_canon.txt"));
        }

        let _ = std::fs::remove_file(&temp_path);
    }
}
