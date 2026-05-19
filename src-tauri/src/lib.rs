#![allow(dead_code)]

mod commands;
mod core;
mod types;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // Phase 1
            commands::ping::ping_start,
            commands::ping::ping_stop,
            commands::traceroute::traceroute_start,
            commands::traceroute::traceroute_stop,
            commands::port_scan::port_scan_start,
            commands::port_scan::port_scan_stop,
            commands::dns::dns_lookup,
            // Phase 2 — LAN
            commands::lan::lan_init,
            commands::lan::lan_shutdown,
            commands::discovery::discovery_peers,
            commands::chat::chat_send,
            commands::chat::chat_broadcast,
            commands::chat::chat_messages,
            commands::chat::chat_history,
            commands::file_transfer::file_send,
            commands::file_transfer::file_accept,
            commands::file_transfer::file_reject,
            commands::file_transfer::file_list,
            commands::file_transfer::file_broadcast,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
