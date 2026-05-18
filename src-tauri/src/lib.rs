mod commands;
mod core;
mod types;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::ping::ping_start,
            commands::ping::ping_stop,
            commands::traceroute::traceroute_start,
            commands::traceroute::traceroute_stop,
            commands::port_scan::port_scan_start,
            commands::port_scan::port_scan_stop,
            commands::dns::dns_lookup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
