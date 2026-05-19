use crate::types::speedtest::SpeedtestResult;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{self, Duration, Instant};

const CHUNK_SIZE: usize = 1024 * 1024; // 1MB chunks

/// Run a full speedtest between this node and a peer.
///
/// The test runs in two phases:
/// 1. Latency/jitter measurement (small timestamped packets)
/// 2. Download & Upload throughput measurement (TCP bulk data)
pub async fn run_speedtest(
    peer_ip: &str,
    port: u16,
    duration_secs: u64,
    on_progress: impl Fn(&str, f64, f64),
) -> SpeedtestResult {
    let peer_addr = format!("{}:{}", peer_ip, port);

    // Phase 1: Latency & Jitter
    on_progress("latency", 0.0, 0.0);
    let (latency_ms, jitter_ms) = measure_latency(&peer_addr).await;

    // Phase 2: Download throughput
    on_progress("download", 0.0, 0.0);
    let download_mbps = measure_download(&peer_addr, duration_secs, |pct, val| {
        on_progress("download", pct, val);
    })
    .await;

    // Phase 3: Upload throughput
    on_progress("upload", 0.0, 0.0);
    let upload_mbps = measure_upload(&peer_addr, duration_secs, |pct, val| {
        on_progress("upload", pct, val);
    })
    .await;

    on_progress("complete", 100.0, 0.0);

    SpeedtestResult {
        download_mbps,
        upload_mbps,
        latency_ms,
        jitter_ms,
        peer_ip: peer_ip.to_string(),
    }
}

/// Measure latency and jitter by sending 10 timestamped ping packets.
async fn measure_latency(peer_addr: &str) -> (f64, f64) {
    let mut rtts = Vec::with_capacity(10);

    for _ in 0..10 {
        match time::timeout(Duration::from_secs(2), ping_once(peer_addr)).await {
            Ok(Ok(rtt)) => rtts.push(rtt),
            _ => rtts.push(-1.0),
        }
        time::sleep(Duration::from_millis(200)).await;
    }

    let valid_rtts: Vec<f64> = rtts.iter().filter(|&&r| r >= 0.0).copied().collect();

    if valid_rtts.is_empty() {
        return (0.0, 0.0);
    }

    let latency_ms = valid_rtts.iter().sum::<f64>() / valid_rtts.len() as f64;

    // Jitter = average deviation from mean
    let jitter_ms = valid_rtts
        .iter()
        .map(|r| (r - latency_ms).abs())
        .sum::<f64>()
        / valid_rtts.len() as f64;

    (latency_ms, jitter_ms)
}

/// Single RTT measurement: connect, send timestamp, receive echo.
async fn ping_once(peer_addr: &str) -> Result<f64, String> {
    let start = Instant::now();
    let mut stream = TcpStream::connect(peer_addr)
        .await
        .map_err(|e| format!("connect: {}", e))?;

    let timestamp = start.elapsed().as_nanos().to_string();
    stream
        .write_all(timestamp.as_bytes())
        .await
        .map_err(|e| format!("write: {}", e))?;

    let mut buf = [0u8; 64];
    stream
        .read(&mut buf)
        .await
        .map_err(|e| format!("read: {}", e))?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    Ok(elapsed)
}

/// Measure download throughput: connect and receive data for the given duration.
async fn measure_download(
    peer_addr: &str,
    duration_secs: u64,
    on_progress: impl Fn(f64, f64),
) -> f64 {
    let total_duration = Duration::from_secs(duration_secs);

    match time::timeout(total_duration + Duration::from_secs(5), async {
        let mut stream = TcpStream::connect(peer_addr).await.map_err(|e| e.to_string())?;
        // Send "DOWNLOAD" signal
        stream.write_all(b"DOWNLOAD").await.map_err(|e| e.to_string())?;

        let mut total_bytes = 0u64;
        let mut buf = vec![0u8; CHUNK_SIZE];
        let start = Instant::now();

        loop {
            let elapsed = start.elapsed();
            if elapsed >= total_duration {
                break;
            }

            match stream.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    total_bytes += n as u64;
                    let pct = (elapsed.as_secs_f64() / total_duration.as_secs_f64() * 100.0)
                        .min(100.0);
                    let mbps = if elapsed.as_secs_f64() > 0.0 {
                        (total_bytes as f64 * 8.0) / elapsed.as_secs_f64() / 1_000_000.0
                    } else {
                        0.0
                    };
                    on_progress(pct, mbps);
                }
                Err(_) => break,
            }
        }

        let elapsed_secs = start.elapsed().as_secs_f64();
        if elapsed_secs > 0.0 {
            Ok::<f64, String>((total_bytes as f64 * 8.0) / elapsed_secs / 1_000_000.0)
        } else {
            Ok::<f64, String>(0.0)
        }
    })
    .await
    {
        Ok(Ok(mbps)) => mbps,
        _ => 0.0,
    }
}

/// Measure upload throughput: connect and send data for the given duration.

/// Measure upload throughput: connect and send data for the given duration.
async fn measure_upload(
    peer_addr: &str,
    duration_secs: u64,
    on_progress: impl Fn(f64, f64),
) -> f64 {
    let total_duration = Duration::from_secs(duration_secs);

    match time::timeout(total_duration + Duration::from_secs(5), async {
        let mut stream = TcpStream::connect(peer_addr).await.map_err(|e| e.to_string())?;
        // Send "UPLOAD" signal (padded to 8 bytes with nulls for consistent framing)
        stream.write_all(b"UPLOAD\0\0").await.map_err(|e| e.to_string())?;

        let data = vec![0u8; CHUNK_SIZE];
        let mut total_sent = 0u64;
        let start = Instant::now();

        loop {
            let elapsed = start.elapsed();
            if elapsed >= total_duration {
                break;
            }

            match stream.write_all(&data).await {
                Ok(()) => {
                    total_sent += CHUNK_SIZE as u64;
                    let pct = (elapsed.as_secs_f64() / total_duration.as_secs_f64() * 100.0)
                        .min(100.0);
                    let mbps = if elapsed.as_secs_f64() > 0.0 {
                        (total_sent as f64 * 8.0) / elapsed.as_secs_f64() / 1_000_000.0
                    } else {
                        0.0
                    };
                    on_progress(pct, mbps);
                }
                Err(_) => break,
            }
        }

        let elapsed_secs = start.elapsed().as_secs_f64();
        if elapsed_secs > 0.0 {
            Ok::<f64, String>((total_sent as f64 * 8.0) / elapsed_secs / 1_000_000.0)
        } else {
            Ok::<f64, String>(0.0)
        }
    })
    .await
    {
        Ok(Ok(mbps)) => mbps,
        _ => 0.0,
    }
}

/// Run the speedtest server side: listen for connections and handle test modes.
pub async fn run_speedtest_server(port: u16, duration_secs: u64) -> Result<(), String> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("bind: {}", e))?;

    // Accept the first connection with a timeout
    let (mut stream, _) = time::timeout(Duration::from_secs(30), listener.accept())
        .await
        .map_err(|_| "Timeout waiting for client connection".to_string())?
        .map_err(|e| format!("accept: {}", e))?;

    // Read test mode (signals are padded to 8 bytes with nulls for consistent framing)
    let mut mode_buf = [0u8; 8];
    time::timeout(Duration::from_secs(10), stream.read_exact(&mut mode_buf))
        .await
        .map_err(|_| "Timeout reading test mode".to_string())?
        .map_err(|e| format!("read mode: {}", e))?;

    // Trim null padding and compare
    let mode = std::str::from_utf8(&mode_buf)
        .unwrap_or("")
        .trim_end_matches('\0');
    let data = vec![0u8; CHUNK_SIZE];

    match mode {
        "DOWNLOAD" => {
            // Server sends data to client (client measures download)
            let total_duration = Duration::from_secs(duration_secs);
            let start = Instant::now();
            loop {
                if start.elapsed() >= total_duration {
                    break;
                }
                if stream.write_all(&data).await.is_err() {
                    break;
                }
            }
        }
        "UPLOAD" => {
            // Server receives data from client (client measures upload)
            let total_duration = Duration::from_secs(duration_secs);
            let mut buf = vec![0u8; CHUNK_SIZE];
            let start = Instant::now();
            loop {
                if start.elapsed() >= total_duration {
                    break;
                }
                // Read once and check for errors or EOF
                match stream.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {} // data received, continue
                }
            }
        }
        _ => {
            // Treat as latency ping: echo back the timestamp
            let mut buf = [0u8; 64];
            if let Ok(n) = stream.read(&mut buf).await {
                let _ = stream.write_all(&buf[..n]).await;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::speedtest::SpeedtestProgress;

    #[tokio::test]
    async fn test_measure_latency_no_server() {
        // Should gracefully handle connection refused
        let (latency, jitter) =
            measure_latency("127.0.0.1:65535").await;
        assert!(latency >= 0.0);
        assert!(jitter >= 0.0);
    }

    #[test]
    fn test_ping_once_timeout() {
        // Connection to unreachable port should fail quickly
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            time::timeout(Duration::from_secs(2), ping_once("127.0.0.1:65535")).await
        });
        assert!(result.is_err() || result.unwrap().is_err());
    }

    // ── Struct and constant validation ──────────────────────────────

    #[test]
    fn test_chunk_size_constant() {
        assert_eq!(CHUNK_SIZE, 1024 * 1024); // 1 MB
    }

    #[test]
    fn test_speedtest_result_serialization_camelcase() {
        let result = SpeedtestResult {
            download_mbps: 100.5,
            upload_mbps: 50.2,
            latency_ms: 10.0,
            jitter_ms: 2.5,
            peer_ip: "192.168.1.1".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("downloadMbps"));
        assert!(json.contains("uploadMbps"));
        assert!(json.contains("latencyMs"));
        assert!(json.contains("jitterMs"));
        assert!(json.contains("peerIp"));
        // Deserialize back
        let deserialized: SpeedtestResult = serde_json::from_str(&json).unwrap();
        assert!((deserialized.download_mbps - 100.5).abs() < 1e-9);
        assert!((deserialized.upload_mbps - 50.2).abs() < 1e-9);
    }

    #[test]
    fn test_speedtest_progress_serialization() {
        let progress = SpeedtestProgress {
            phase: "download".to_string(),
            percent: 50.0,
            current_value: 25.0,
        };
        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("\"phase\""));
        assert!(json.contains("\"percent\""));
        assert!(json.contains("\"currentValue\""));

        let deserialized: SpeedtestProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.phase, "download");
        assert!((deserialized.percent - 50.0).abs() < 1e-9);
    }

    #[test]
    fn test_speedtest_result_default_values() {
        let result = SpeedtestResult {
            download_mbps: 0.0,
            upload_mbps: 0.0,
            latency_ms: 0.0,
            jitter_ms: 0.0,
            peer_ip: "".to_string(),
        };
        assert_eq!(result.download_mbps, 0.0);
        assert_eq!(result.upload_mbps, 0.0);
        assert_eq!(result.latency_ms, 0.0);
        assert_eq!(result.jitter_ms, 0.0);
        assert!(result.peer_ip.is_empty());
    }

    // ── measure_latency edge cases ──────────────────────────────────

    #[tokio::test]
    async fn test_measure_latency_no_nan_values() {
        let (latency, jitter) = measure_latency("127.0.0.1:1").await;
        assert!(!latency.is_nan());
        assert!(!jitter.is_nan());
        assert!(latency.is_finite());
        assert!(jitter.is_finite());
    }

    #[tokio::test]
    async fn test_measure_latency_unreachable_port() {
        // Using a very low port that should not be in use
        let (latency, jitter) = measure_latency("127.0.0.1:0").await;
        // Should return (0.0, 0.0) since no valid RTTs were collected
        assert_eq!(latency, 0.0);
        assert_eq!(jitter, 0.0);
    }

    // ── measure_download / measure_upload timeout behavior ──────────

    #[tokio::test]
    async fn test_measure_download_no_server() {
        let calls = std::sync::atomic::AtomicU64::new(0);
        let result = measure_download("127.0.0.1:65534", 1, |_pct, _val| {
            calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        })
        .await;
        assert_eq!(result, 0.0); // No server, should return 0
    }

    #[tokio::test]
    async fn test_measure_upload_no_server() {
        let result = measure_upload("127.0.0.1:65533", 1, |_pct, _val| {})
            .await;
        assert_eq!(result, 0.0);
    }
}
