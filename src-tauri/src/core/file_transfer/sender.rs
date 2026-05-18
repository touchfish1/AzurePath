use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub struct FileSender {
    running: Arc<AtomicBool>,
    /// Active transfer progress: file_id -> (bytes_sent, total_size)
    pub(crate) active: Arc<Mutex<std::collections::HashMap<String, (u64, u64)>>>,
}

impl FileSender {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(true)),
            active: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Send a file to a peer's file receiver.
    /// - `peer_addr`: IP address of the peer (receiver listens on 0.0.0.0:receiver_port)
    /// - `receiver_port`: the port the peer's receiver is listening on
    /// - `file_id`: unique ID for this transfer
    /// - `file_path`: path to the file to send
    pub async fn send_file(
        self: &Arc<Self>,
        peer_addr: &str,
        receiver_port: u16,
        file_id: &str,
        file_path: &Path,
    ) -> Result<(), String> {
        let addr = format!("{}:{}", peer_addr, receiver_port);
        println!("[file] Sending {} to {}", file_id, addr);

        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| format!("Failed to connect to file receiver: {}", e))?;

        // Open file and get metadata
        let mut file = tokio::fs::File::open(file_path)
            .await
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let metadata = file
            .metadata()
            .await
            .map_err(|e| format!("Failed to read metadata: {}", e))?;
        let file_size = metadata.len();

        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid filename")?;

        // Send file_id
        write_string(&mut stream, file_id).await?;
        // Send total size
        write_u64(&mut stream, file_size).await?;
        // Send filename
        write_string(&mut stream, filename).await?;

        // Track progress
        self.active.lock().await.insert(file_id.to_string(), (0, file_size));

        // Stream file data
        let mut buf = vec![0u8; 64 * 1024];
        let mut sent: u64 = 0;

        loop {
            if !self.running.load(Ordering::SeqCst) {
                // Cancelled
                self.active.lock().await.remove(file_id);
                return Err("Transfer cancelled".to_string());
            }

            let n = file
                .read(&mut buf)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))?;

            if n == 0 {
                break; // EOF
            }

            stream
                .write_all(&buf[..n])
                .await
                .map_err(|e| format!("Failed to write to stream: {}", e))?;

            sent += n as u64;
            self.active
                .lock()
                .await
                .insert(file_id.to_string(), (sent, file_size));
        }

        self.active.lock().await.remove(file_id);
        println!("[file] Sent {}: {} bytes", file_id, sent);

        Ok(())
    }
}

async fn write_string(stream: &mut TcpStream, s: &str) -> Result<(), String> {
    let bytes = s.as_bytes();
    write_u64(stream, bytes.len() as u64).await?;
    stream
        .write_all(bytes)
        .await
        .map_err(|e| format!("Failed to write string: {}", e))?;
    Ok(())
}

async fn write_u64(stream: &mut TcpStream, val: u64) -> Result<(), String> {
    let buf = val.to_be_bytes();
    stream
        .write_all(&buf)
        .await
        .map_err(|e| format!("Failed to write u64: {}", e))?;
    Ok(())
}
