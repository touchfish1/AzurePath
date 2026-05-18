use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

pub struct FileReceiver {
    running: Arc<AtomicBool>,
    download_dir: PathBuf,
    /// Active transfer handles: file_id -> (bytes_received, total_size)
    pub(crate) active: Arc<Mutex<std::collections::HashMap<String, (u64, u64)>>>,
}

impl FileReceiver {
    pub fn new() -> Result<Self, String> {
        let home = home_dir().ok_or("Cannot find home directory")?;
        let download_dir = home.join("AzurePath").join("downloads");
        std::fs::create_dir_all(&download_dir)
            .map_err(|e| format!("Failed to create download dir: {}", e))?;

        Ok(Self {
            running: Arc::new(AtomicBool::new(true)),
            download_dir,
            active: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }

    /// Start listening on a dynamic port for incoming file transfers.
    /// Returns the port number.
    pub async fn start_listener(self: &Arc<Self>) -> Result<u16, String> {
        let listener = TcpListener::bind("0.0.0.0:0")
            .await
            .map_err(|e| format!("Failed to bind file transfer listener: {}", e))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("Failed to get local addr: {}", e))?
            .port();

        println!("[file] Receiver listening on port {}", port);

        let this = self.clone();
        tokio::spawn(async move {
            loop {
                if !this.running.load(Ordering::SeqCst) {
                    break;
                }
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        println!("[file] Incoming file transfer from {}", addr);
                        let this_clone = this.clone();
                        tokio::spawn(async move {
                            this_clone.receive_file(stream).await;
                        });
                    }
                    Err(e) => eprintln!("[file] Accept error: {}", e),
                }
            }
        });

        Ok(port)
    }

    async fn receive_file(self: &Arc<Self>, mut stream: TcpStream) {
        // Read file_id (length-prefixed string)
        let file_id = match read_string(&mut stream).await {
            Ok(id) => id,
            Err(e) => {
                eprintln!("[file] Failed to read file_id: {}", e);
                return;
            }
        };

        // Read total file size
        let total_size = match read_u64(&mut stream).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[file] Failed to read file size: {}", e);
                return;
            }
        };

        // Read filename
        let filename = match read_string(&mut stream).await {
            Ok(name) => name,
            Err(e) => {
                eprintln!("[file] Failed to read filename: {}", e);
                return;
            }
        };

        let dest_path = self.download_dir.join(&filename);
        println!(
            "[file] Receiving {} ({} bytes) -> {:?}",
            filename, total_size, dest_path
        );

        self.active.lock().await.insert(file_id.clone(), (0, total_size));

        // Stream data to file
        match tokio::fs::File::create(&dest_path).await {
            Ok(mut file) => {
                let mut received = 0u64;
                let mut buf = vec![0u8; 64 * 1024];

                while received < total_size {
                    let remaining = (total_size - received) as usize;
                    let to_read = buf.len().min(remaining);
                    match stream.read_exact(&mut buf[..to_read]).await {
                        Ok(_) => {
                            if let Err(e) = file.write_all(&buf[..to_read]).await {
                                eprintln!("[file] Write error: {}", e);
                                break;
                            }
                            received += to_read as u64;
                            let mut active = self.active.lock().await;
                            active.insert(file_id.clone(), (received, total_size));
                        }
                        Err(e) => {
                            eprintln!("[file] Read error during transfer: {}", e);
                            break;
                        }
                    }
                }

                if received == total_size {
                    println!("[file] Transfer complete: {}", filename);
                }
            }
            Err(e) => eprintln!("[file] Failed to create file {:?}: {}", dest_path, e),
        }

        self.active.lock().await.remove(&file_id);
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(PathBuf::from)
}

async fn read_string(stream: &mut TcpStream) -> Result<String, String> {
    let len = read_u64(stream).await? as usize;
    let mut buf = vec![0u8; len];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|e| format!("Failed to read string: {}", e))?;
    String::from_utf8(buf).map_err(|e| format!("Invalid UTF-8: {}", e))
}

async fn read_u64(stream: &mut TcpStream) -> Result<u64, String> {
    let mut buf = [0u8; 8];
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|e| format!("Failed to read u64: {}", e))?;
    Ok(u64::from_be_bytes(buf))
}
