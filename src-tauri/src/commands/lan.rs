use crate::core::connection::ConnectionManager;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// Initialize all LAN services: discovery, connection, chat, file transfer.
#[tauri::command]
pub async fn lan_init(app: AppHandle) -> Result<(), String> {
    // 1. Start connection manager (TCP listener)
    let conn_mgr = Arc::new(ConnectionManager::new());
    conn_mgr.clone().start_listener().await?;
    crate::commands::chat::set_conn_mgr(conn_mgr.clone());

    // 2. Start file transfer service (receiver on dynamic port)
    crate::commands::file_transfer::file_transfer_init(app.clone()).await?;

    // 3. Wire connection manager into file transfer service for progress reporting
    crate::commands::file_transfer::set_file_conn_mgr(conn_mgr.clone()).await;

    // 4. Start chat service (subscribes to connection frames)
    crate::commands::chat::chat_init(app.clone()).await?;

    // 5. Start discovery service (UDP broadcast + heartbeat)
    //    This also triggers connection to discovered peers.
    crate::commands::discovery::discovery_start(app.clone()).await?;

    println!("[lan] All services initialized successfully");
    let _ = app.emit("lan:ready", serde_json::json!({ "status": "ok" }));

    // 5. When new peers are discovered via discovery, connect to them via TCP
    let conn_mgr_clone = conn_mgr.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            let peers = match crate::commands::discovery::DISCOVERY.get() {
                Some(d) => d.peer_table().online_peers().await,
                None => continue,
            };

            let connected = conn_mgr_clone.connected_peers().await;

            for peer in &peers {
                if !connected.contains(&peer.id) {
                    // Don't connect to self
                    if peer.id == crate::core::discovery::my_id().await {
                        continue;
                    }
                    println!("[lan] Auto-connecting to peer {} at {}", peer.id, peer.ip);
                    let mgr = conn_mgr_clone.clone();
                    let pid = peer.id.clone();
                    let pip = peer.ip.clone();
                    tokio::spawn(async move {
                        if let Err(e) = mgr.connect_to_peer(&pid, &pip).await {
                            eprintln!("[lan] Failed to connect to {}: {}", pid, e);
                        }
                    });
                }
            }
        }
    });

    Ok(())
}

/// Shutdown all LAN services.
#[tauri::command]
pub async fn lan_shutdown() -> Result<(), String> {
    if let Some(d) = crate::commands::discovery::DISCOVERY.get() {
        d.stop();
    }
    Ok(())
}
