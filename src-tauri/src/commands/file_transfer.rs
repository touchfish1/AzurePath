use crate::core::connection::IncomingFrame;
use crate::core::file_transfer::FileTransferService;
use crate::types::file_transfer::FileTransfer;
use std::sync::OnceLock;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

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
            from,
        } => {
            // Store incoming request info; frontend will decide to accept/reject
            svc.register_incoming(file_id, filename, *size, from).await;
            let _ = app.emit(
                "file:request",
                serde_json::json!({
                    "fileId": file_id,
                    "filename": filename,
                    "size": size,
                    "from": from,
                }),
            );
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

#[tauri::command]
pub async fn file_send(
    target: String,
    path: String,
) -> Result<String, String> {
    let svc = FILE_SVC.get().ok_or("File transfer not initialized")?;

    // Get file metadata
    let file_path = std::path::PathBuf::from(&path);
    let metadata = std::fs::metadata(&file_path)
        .map_err(|e| format!("Cannot read file: {}", e))?;
    let file_size = metadata.len();
    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?
        .to_string();

    // Get peer info for the receiver port
    let peer = match crate::commands::discovery::DISCOVERY.get() {
        Some(d) => d.get_peer(&target).await,
        None => None,
    };
    let peer_addr = peer
        .as_ref()
        .map(|p| p.ip.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let receiver_port = match svc.get_receiver_port().await {
        Some(p) => p,
        None => return Err("File receiver not ready".to_string()),
    };

    // Send file request via chat connection
    let conn_mgr = match crate::commands::chat::CONN_MGR.get() {
        Some(c) => c,
        None => return Err("Connection manager not initialized".to_string()),
    };

    conn_mgr
        .send(
            &target,
            &crate::types::chat::Frame::FileRequest {
                file_id: "pending".to_string(),
                filename: filename.clone(),
                size: file_size,
                from: crate::core::discovery::my_id().await,
            },
        )
        .await?;

    // Initiate the actual transfer
    svc.initiate_transfer(
        &target,
        &peer_addr,
        receiver_port,
        &path,
        filename,
        file_size,
    )
    .await
}

#[tauri::command]
pub async fn file_accept(file_id: String, receiver_port: u16) -> Result<(), String> {
    // Send file_response with accept=true via chat connection
    let conn_mgr = match crate::commands::chat::CONN_MGR.get() {
        Some(c) => c,
        None => return Err("Connection manager not initialized".to_string()),
    };

    // We need to know which peer sent this file request.
    // For now, broadcast the response (the sender will recognize its file_id).
    conn_mgr
        .broadcast(&crate::types::chat::Frame::FileResponse {
            file_id,
            accepted: true,
            data_port: receiver_port,
        })
        .await;

    Ok(())
}

#[tauri::command]
pub async fn file_reject(file_id: String) -> Result<(), String> {
    let conn_mgr = match crate::commands::chat::CONN_MGR.get() {
        Some(c) => c,
        None => return Err("Connection manager not initialized".to_string()),
    };

    conn_mgr
        .broadcast(&crate::types::chat::Frame::FileResponse {
            file_id,
            accepted: false,
            data_port: 0,
        })
        .await;

    Ok(())
}

#[tauri::command]
pub async fn file_list() -> Result<Vec<FileTransfer>, String> {
    let svc = FILE_SVC.get().ok_or("File transfer not initialized")?;
    Ok(svc.list_transfers().await)
}
