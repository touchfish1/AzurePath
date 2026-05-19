use chrono::Local;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::core::ssh;
use crate::types::ssh::SshSession;

#[tauri::command]
pub async fn ssh_connect(
    app: AppHandle,
    id: String,
    host: String,
    port: u16,
    username: String,
    password: String,
) -> Result<(), String> {
    let session_id = if id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        id
    };

    let connected_at = Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string();

    // Create communication channel
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();

    // Register session handle BEFORE spawning the blocking task,
    // so that other commands can find it immediately.
    let handle = ssh::SshSessionHandle {
        cmd_tx: cmd_tx.clone(),
        host: host.clone(),
        port,
        username: username.clone(),
        connected_at: connected_at.clone(),
    };
    ssh::register_session(session_id.clone(), handle).await;

    // Spawn the blocking SSH session runner
    ssh::spawn_session(
        app.clone(),
        session_id.clone(),
        host,
        port,
        username,
        password,
        cmd_rx,
        connected_at,
    );

    // Return the session id in the event payload so the frontend can capture it
    let _ = app.emit(
        "ssh:session_created",
        serde_json::json!({ "sessionId": session_id }),
    );

    Ok(())
}

#[tauri::command]
pub async fn ssh_disconnect(id: String) -> Result<(), String> {
    if let Some(cmd_tx) = ssh::get_command_sender(&id).await {
        let _ = cmd_tx.send(ssh::SshCommand::Disconnect);
    }
    ssh::unregister_session(&id).await;
    Ok(())
}

#[tauri::command]
pub async fn ssh_send_input(id: String, data: String) -> Result<(), String> {
    let cmd_tx = ssh::get_command_sender(&id)
        .await
        .ok_or_else(|| format!("SSH session '{}' not found", id))?;

    use base64::Engine;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| format!("Failed to decode base64 input: {}", e))?;

    cmd_tx
        .send(ssh::SshCommand::Input(raw))
        .map_err(|_| format!("SSH session '{}' is closed", id))?;

    Ok(())
}

#[tauri::command]
pub async fn ssh_resize(id: String, cols: u32, rows: u32) -> Result<(), String> {
    let cmd_tx = ssh::get_command_sender(&id)
        .await
        .ok_or_else(|| format!("SSH session '{}' not found", id))?;

    cmd_tx
        .send(ssh::SshCommand::Resize(cols, rows))
        .map_err(|_| format!("SSH session '{}' is closed", id))?;

    Ok(())
}

#[tauri::command]
pub async fn ssh_list_sessions() -> Result<Vec<SshSession>, String> {
    Ok(ssh::list_sessions().await)
}
