use tauri::{AppHandle, Emitter};

use crate::core::speedtest;
use crate::types::speedtest::{SpeedtestProgress, SpeedtestResult};

#[tauri::command]
pub async fn start_speedtest(
    app: AppHandle,
    peer_ip: String,
    port: u16,
    duration_secs: u64,
    mode: String, // "client" or "server"
) -> Result<String, String> {
    let app_clone = app.clone();
    let peer_ip_clone = peer_ip.clone();

    tauri::async_runtime::spawn(async move {
        if mode == "server" {
            // Server mode: just listen and serve data
            match speedtest::run_speedtest_server(port, duration_secs).await {
                Ok(()) => {
                    let _ = app_clone.emit(
                        "speedtest:complete",
                        SpeedtestResult {
                            download_mbps: 0.0,
                            upload_mbps: 0.0,
                            latency_ms: 0.0,
                            jitter_ms: 0.0,
                            peer_ip: peer_ip_clone,
                        },
                    );
                }
                Err(e) => {
                    let _ = app_clone.emit(
                        "speedtest:error",
                        serde_json::json!({ "error": e }),
                    );
                }
            }
        } else {
            // Client mode: run the full test
            let result = speedtest::run_speedtest(
                &peer_ip_clone,
                port,
                duration_secs,
                &|phase: &str, percent, current_value| {
                    let _ = app_clone.emit(
                        "speedtest:progress",
                        SpeedtestProgress {
                            phase: phase.to_string(),
                            percent,
                            current_value,
                        },
                    );
                },
            )
            .await;

            let _ = app_clone.emit("speedtest:complete", &result);
        }
    });

    Ok("speedtest_started".to_string())
}
