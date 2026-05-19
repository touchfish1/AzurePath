mod commands;
mod core;
mod types;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None::<Vec<&'static str>>))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    use tauri_plugin_global_shortcut::{Code, Modifiers};
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed
                        && shortcut.matches(Modifiers::ALT | Modifiers::CONTROL, Code::KeyA)
                    {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Initialize activity store
            let _ = commands::history::init_activity_store();

            use tauri::menu::{MenuBuilder, MenuItemBuilder};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            // Build tray menu
            let show = MenuItemBuilder::with_id("show", "显示窗口").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show)
                .separator()
                .item(&quit)
                .build()?;

            // Create system tray
            TrayIconBuilder::new()
                .tooltip("AzurePath")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button,
                        button_state,
                        ..
                    } = event
                    {
                        if button == MouseButton::Left
                            && button_state == MouseButtonState::Up
                        {
                            if let Some(window) = tray.app_handle().get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // Register global shortcut (Ctrl+Alt+A)
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            app.global_shortcut().register(
                tauri_plugin_global_shortcut::Shortcut::new(
                    Some(tauri_plugin_global_shortcut::Modifiers::ALT | tauri_plugin_global_shortcut::Modifiers::CONTROL),
                    tauri_plugin_global_shortcut::Code::KeyA,
                ),
            )?;

            Ok(())
        })
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
            commands::chat::chat_search,
            commands::chat::chat_delete,
            commands::chat::chat_clear,
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
            commands::clipboard::clipboard_export,
            commands::clipboard::clipboard_sources,
            commands::clipboard::clipboard_set_limit,
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
            // Phase 5 — Activity History
            commands::history::activity_list,
            commands::history::activity_search,
            commands::history::activity_delete,
            commands::history::activity_clear,
            commands::history::activity_log,
            // Phase 6 — Settings
            commands::settings::get_settings,
            commands::settings::save_settings,
            // Phase 7 — Export
            commands::export::export_chat,
            commands::export::export_clipboard,
            commands::export::export_settings,
            // Phase 8 — Network Tools
            commands::whois::whois_lookup,
            commands::http_check::http_check,
            commands::ssl_check::ssl_check,
            commands::mac_lookup::mac_lookup,
            // Phase 9 — WOL
            commands::wol::wol_send,
            commands::wol::wol_save,
            commands::wol::wol_list,
            commands::wol::wol_delete,
            // Phase 10 — Direction A
            commands::speedtest::start_speedtest,
            commands::preset::save_preset,
            commands::preset::load_presets,
            commands::preset::delete_preset,
            // Phase 12 — Monitor
            commands::monitor::monitor_start,
            commands::monitor::monitor_stop,
            commands::monitor::monitor_status,
            commands::monitor::monitor_add_target,
            commands::monitor::monitor_list_targets,
            commands::monitor::monitor_delete_target,
            commands::monitor::monitor_get_history,
            commands::monitor::monitor_get_all_recent_history,
            // Phase 13 — Export Data
            commands::export_data::save_file,
            // Phase 14 — mDNS Discovery
            commands::mdns::mdns_discover,
            // Phase 15 — Bandwidth Monitor
            commands::bandwidth::get_interfaces,
            commands::bandwidth::start_bandwidth_monitor,
            commands::bandwidth::stop_bandwidth_monitor,
            // Phase 16 — Report Export
            commands::report::save_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
