use crate::core::discovery::DiscoveryService;
use std::sync::OnceLock;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

pub(crate) static DISCOVERY: OnceLock<Arc<DiscoveryService>> = OnceLock::new();

#[tauri::command]
pub async fn discovery_start(app: AppHandle) -> Result<(), String> {
    // Prevent double start
    if DISCOVERY.get().is_some() {
        return Ok(());
    }

    let service = Arc::new(DiscoveryService::new());
    service.clone().start().await?;
    DISCOVERY.set(service).map_err(|_| "Already initialized".to_string())?;

    // Spawn a task to forward peer events to the frontend
    let app_clone = app.clone();
    tokio::spawn(async move {
        // Periodically check peer list changes and emit events
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

            if let Some(disc) = DISCOVERY.get() {
                let peers = disc.peer_table().list().await;
                let _ = app_clone.emit("peer:list", &peers);
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn discovery_peers() -> Result<Vec<crate::types::discovery::PeerInfo>, String> {
    match DISCOVERY.get() {
        Some(service) => Ok(service.peer_table().list().await),
        None => Ok(Vec::new()),
    }
}

#[tauri::command]
pub async fn discovery_stop() -> Result<(), String> {
    if let Some(service) = DISCOVERY.get() {
        service.stop();
    }
    Ok(())
}
