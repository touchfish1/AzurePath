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
#[allow(dead_code)]
pub async fn discovery_stop() -> Result<(), String> {
    if let Some(service) = DISCOVERY.get() {
        service.stop();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_peers_before_start_returns_empty() {
        let result = discovery_peers().await;
        assert!(result.is_ok(), "peers should not error before start");
        assert!(result.unwrap().is_empty(), "peer list should be empty before start");
    }

    #[tokio::test]
    async fn test_stop_before_start_is_noop() {
        let result = discovery_stop().await;
        assert!(result.is_ok(), "stop should not error before start");
    }

    #[test]
    fn test_static_not_initialized() {
        assert!(DISCOVERY.get().is_none(), "DISCOVERY must be None before discovery_start");
    }
}
