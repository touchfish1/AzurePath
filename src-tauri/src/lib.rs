mod commands;
mod core;
mod types;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            commands::file_transfer::get_file_download_url,
            // Phase 3 — Clipboard
            commands::clipboard::clipboard_start,
            commands::clipboard::clipboard_stop,
            commands::clipboard::clipboard_list,
            commands::clipboard::clipboard_delete,
            commands::clipboard::clipboard_toggle_favorite,
            commands::clipboard::clipboard_copy,
            commands::clipboard::clipboard_clear,
            commands::clipboard::clipboard_get_interval,
            commands::clipboard::clipboard_set_interval,
            // Phase 4 — Network Sniffer
            commands::network_sniffer::sniffer_start,
            commands::network_sniffer::sniffer_stop,
            commands::network_sniffer::sniffer_list,
            commands::network_sniffer::sniffer_export,
            commands::network_sniffer::sniffer_presets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
