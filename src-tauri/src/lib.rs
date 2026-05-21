mod commands;
mod core;
mod types;

pub use core::log_buffer::LogLayer;
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
            // Initialize remote shell store
            if let Err(e) = tauri::async_runtime::block_on(commands::remote_shell::remote_shell_init()) {
                eprintln!("[azurepath] remote_shell init warning: {e}");
            }
            // Initialize remote desktop store
            if let Err(e) = tauri::async_runtime::block_on(commands::remote_desktop::remote_desktop_init()) {
                eprintln!("[azurepath] remote_desktop init warning: {e}");
            }
            // Initialize snmp store
            if let Err(e) = tauri::async_runtime::block_on(commands::snmp::snmp_init()) {
                eprintln!("[azurepath] snmp init warning: {e}");
            }
            // Initialize topology store
            let _ = commands::topology::topology_init();

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
            if let Err(e) = app.global_shortcut().register(
                tauri_plugin_global_shortcut::Shortcut::new(
                    Some(tauri_plugin_global_shortcut::Modifiers::ALT | tauri_plugin_global_shortcut::Modifiers::CONTROL),
                    tauri_plugin_global_shortcut::Code::KeyA,
                ),
            ) {
                eprintln!("[azurepath] global shortcut register warning: {e}");
            }

            Ok(())
        })
        // single_instance is not available in this version of Tauri 2.
        // Multi-instance prevention is handled by the OS or through the
        // tauri-plugin-single-instance if needed.
        .invoke_handler(tauri::generate_handler![
            // Phase 1
            commands::ping::ping_start,
            commands::ping::ping_stop,
            commands::traceroute::traceroute_start,
            commands::traceroute::traceroute_stop,
            commands::topology::discover_topology,
            commands::topology::cancel_topology_discovery,
            // Topology enhanced commands
            commands::topology::compute_topology_layout,
            commands::topology::topology_save_snapshot,
            commands::topology::topology_list_snapshots,
            commands::topology::topology_load_snapshot,
            commands::topology::topology_delete_snapshot,
            commands::topology::topology_compare_snapshots,
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
            commands::app_settings::settings_init,
            commands::app_settings::settings_get,
            commands::app_settings::settings_set,
            commands::app_settings::settings_delete,
            commands::app_settings::settings_get_all,
            commands::app_settings::settings_clear,
            // Operation History
            commands::operation_history::history_init,
            commands::operation_history::history_list,
            commands::operation_history::history_get,
            commands::operation_history::history_delete,
            commands::operation_history::history_clear,
            commands::operation_history::history_set_max_entries,
            commands::operation_history::history_get_max_entries,
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
            // Phase 14 — MTR Route Tracing
            commands::mtr::mtr_start,
            commands::mtr::mtr_stop,
            // Phase 15 — mDNS Discovery
            commands::mdns::mdns_discover,
            // Phase 16 — Bandwidth Monitor
            commands::bandwidth::get_interfaces,
            commands::bandwidth::start_bandwidth_monitor,
            commands::bandwidth::stop_bandwidth_monitor,
            // Phase 17 — Report Export
            commands::report::save_report,
            // Phase 18 — Logs
            commands::logs::get_logs,
            commands::logs::clear_logs,
            // Phase 19 — API Test
            commands::api_test::send_api_request,
            commands::api_test::list_api_requests,
            commands::api_test::save_api_request,
            commands::api_test::delete_api_request,
            // API Test WebSocket
            commands::api_test::ws_connect,
            commands::api_test::ws_send,
            commands::api_test::ws_close,
            commands::api_test::ws_get_messages,
            commands::api_test::ws_clear_messages,
            // API Test Environment
            commands::api_test::env_list,
            commands::api_test::env_save,
            commands::api_test::env_delete,
            // API Test Collections
            commands::api_test::collection_list,
            commands::api_test::collection_save,
            commands::api_test::collection_delete,
            // API Test Code Generation
            commands::api_test::generate_http_code,
            // Phase 18 — Backup & Restore
            commands::backup::backup_all_data,
            commands::backup::list_backups,
            commands::backup::restore_backup,
            commands::backup::delete_backup,
            // Phase 19 — Target Group Management
            commands::target_group::list_target_groups,
            commands::target_group::get_target_group,
            commands::target_group::save_target_group,
            commands::target_group::delete_target_group,
            // Bookmarks
            commands::bookmark::list_bookmarks,
            commands::bookmark::add_bookmark,
            commands::bookmark::delete_bookmark,
            // Subnet Calculator
            commands::subnet::calculate_subnet,
            commands::subnet::split_subnet,
            // Remote Shell
            commands::remote_shell::remote_shell_init,
            commands::remote_shell::remote_shell_list_sessions,
            commands::remote_shell::remote_shell_get_session,
            commands::remote_shell::remote_shell_create_session,
            commands::remote_shell::remote_shell_update_session,
            commands::remote_shell::remote_shell_delete_session,
            commands::remote_shell::remote_shell_connect,
            commands::remote_shell::remote_shell_disconnect,
            commands::remote_shell::remote_shell_send_input,
            commands::remote_shell::remote_shell_pull_output,
            commands::remote_shell::remote_shell_resize,
            commands::remote_shell::remote_shell_list_summaries,
            commands::remote_shell::remote_shell_list_sftp,
            commands::remote_shell::remote_shell_read_sftp_text,
            commands::remote_shell::remote_shell_save_sftp_text,
            commands::remote_shell::remote_shell_get_metrics,
            commands::remote_shell::remote_shell_list_environments,
            commands::remote_shell::remote_shell_create_environment,
            commands::remote_shell::remote_shell_list_db_connections,
            commands::remote_shell::remote_shell_create_db_connection,
            commands::remote_shell::remote_shell_delete_db_connection,
            commands::remote_shell::remote_shell_test_db_connection,
            commands::remote_shell::remote_shell_mysql_list_databases,
            commands::remote_shell::remote_shell_mysql_list_tables,
            commands::remote_shell::remote_shell_mysql_describe_table,
            commands::remote_shell::remote_shell_mysql_execute_query,
            commands::remote_shell::remote_shell_pg_list_databases,
            commands::remote_shell::remote_shell_pg_list_tables,
            commands::remote_shell::remote_shell_pg_execute_query,
            commands::remote_shell::remote_shell_redis_list_keys,
            commands::remote_shell::remote_shell_redis_get_value,
            commands::remote_shell::remote_shell_redis_set_value,
            commands::remote_shell::remote_shell_redis_set_ttl,
            // Remote Desktop
            commands::remote_desktop::rd_list_sessions,
            commands::remote_desktop::rd_create_session,
            commands::remote_desktop::rd_update_session,
            commands::remote_desktop::rd_delete_session,
            commands::remote_desktop::rd_connect,
            commands::remote_desktop::rd_disconnect,
            commands::remote_desktop::rd_resize,
            commands::remote_desktop::rd_send_key,
            commands::remote_desktop::rd_send_mouse,
            // System Info
            commands::system_info::get_local_network_info,
            // Remote Desktop Clipboard
            commands::remote_desktop::rd_push_clipboard,
            // SNMP
            commands::snmp::snmp_discover,
            commands::snmp::snmp_list_devices,
            commands::snmp::snmp_delete_device,
            commands::snmp::snmp_get_interfaces,
            commands::snmp::snmp_get_arp_table,
            commands::snmp::snmp_get_history,
            commands::snmp::snmp_start_collect,
            commands::snmp::snmp_stop_collect,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
