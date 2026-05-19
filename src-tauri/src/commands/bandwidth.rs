use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter};

use crate::core::bandwidth;
use crate::core::bandwidth::CounterSnapshot;
use crate::types::bandwidth::InterfaceInfo;

static MONITOR_FLAGS: LazyLock<Mutex<HashMap<String, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn set_monitor_flag(name: &str, value: bool) {
    if let Ok(mut flags) = MONITOR_FLAGS.lock() {
        flags.insert(name.to_string(), value);
    }
}

fn get_monitor_flag(name: &str) -> bool {
    MONITOR_FLAGS
        .lock()
        .map(|flags| flags.get(name).copied().unwrap_or(false))
        .unwrap_or(false)
}

/// Get the list of available network interfaces.
#[tauri::command]
pub fn get_interfaces() -> Result<Vec<InterfaceInfo>, String> {
    bandwidth::get_interfaces()
}

/// Start the bandwidth monitor.
///
/// Spawns a background task that samples network interface counters every
/// second and emits `bandwidth:data` events with `Vec<BandwidthSample>`.
#[tauri::command]
pub async fn start_bandwidth_monitor(app: AppHandle) -> Result<(), String> {
    let monitor_id = "default".to_string();

    if get_monitor_flag(&monitor_id) {
        return Err("Bandwidth monitor is already running".to_string());
    }

    set_monitor_flag(&monitor_id, true);

    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        // Initial counter snapshot via spawn_blocking (wmic is blocking).
        let mut previous: HashMap<String, CounterSnapshot> = match tokio::task::spawn_blocking(bandwidth::get_counters).await.unwrap_or(Err("spawn_blocking failed".into())) {
            Ok(c) => c,
            Err(e) => {
                let _ = app_clone.emit(
                    "bandwidth:error",
                    serde_json::json!({ "error": e }),
                );
                return;
            }
        };

        loop {
            // Check if we should stop.
            if !get_monitor_flag(&monitor_id) {
                break;
            }

            // Sleep for 1 second.
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            // Check again after sleep.
            if !get_monitor_flag(&monitor_id) {
                break;
            }

            // Get new counters via spawn_blocking (wmic is blocking).
            let current = match tokio::task::spawn_blocking(bandwidth::get_counters).await.unwrap_or(Err("spawn_blocking failed".into())) {
                Ok(c) => c,
                Err(e) => {
                    let _ = app_clone.emit(
                        "bandwidth:error",
                        serde_json::json!({ "error": e }),
                    );
                    break;
                }
            };

            // Compute samples.
            let samples = bandwidth::compute_samples(&previous, &current, 1.0);

            // Emit event.
            let _ = app_clone.emit("bandwidth:data", &samples);

            previous = current;
        }

        set_monitor_flag(&monitor_id, false);
    });

    Ok(())
}

/// Stop the bandwidth monitor.
#[tauri::command]
pub async fn stop_bandwidth_monitor() -> Result<(), String> {
    set_monitor_flag("default", false);
    Ok(())
}
