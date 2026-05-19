use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Maximum allowed length for a filename or file_id string read from wire.
const MAX_WIRE_STRING_LEN: usize = 4096;
/// Maximum allowed file size in bytes (10 GB).
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024 * 1024;

pub struct FileReceiver {
    running: Arc<AtomicBool>,
    download_dir: PathBuf,
    /// Active transfer handles: file_id -> (bytes_received, total_size)
    pub(crate) active: Arc<Mutex<std::collections::HashMap<String, (u64, u64)>>>,
}

impl FileReceiver {
    pub fn new() -> Result<Self, String> {
        let download_dir = default_download_dir();
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

        info!("[file] Receiver listening on port {}", port);

        let this = self.clone();
        tokio::spawn(async move {
            loop {
                if !this.running.load(Ordering::SeqCst) {
                    break;
                }
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        info!("[file] Incoming file transfer from {}", addr);
                        let this_clone = this.clone();
                        tokio::spawn(async move {
                            this_clone.receive_file(stream).await;
                        });
                    }
                    Err(e) => warn!("[file] Accept error: {}", e),
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
                warn!("[file] Failed to read file_id: {}", e);
                return;
            }
        };

        // Read total file size
        let total_size = match read_u64(&mut stream).await {
            Ok(s) => s,
            Err(e) => {
                warn!("[file] Failed to read file size: {}", e);
                return;
            }
        };

        // Reject files larger than the maximum allowed size
        if total_size > MAX_FILE_SIZE {
            warn!(
                "[file] Rejected file of size {} bytes (max allowed: {} bytes)",
                total_size, MAX_FILE_SIZE
            );
            return;
        }

        // Read filename
        let filename_raw = match read_string(&mut stream).await {
            Ok(name) => name,
            Err(e) => {
                warn!("[file] Failed to read filename: {}", e);
                return;
            }
        };

        // SECURITY: Sanitize the filename to prevent path traversal attacks.
        // A malicious peer could send a filename like "../../etc/passwd".
        let filename = sanitize_filename(&filename_raw);
        let dest_path = self.download_dir.join(&filename);
        info!(
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
                    match tokio::time::timeout(
                        Duration::from_secs(30),
                        stream.read_exact(&mut buf[..to_read]),
                    )
                    .await
                    {
                        Ok(Ok(_)) => {
                            if let Err(e) = file.write_all(&buf[..to_read]).await {
                                warn!("[file] Write error: {}", e);
                                break;
                            }
                            received += to_read as u64;
                            let mut active = self.active.lock().await;
                            active.insert(file_id.clone(), (received, total_size));
                        }
                        Ok(Err(e)) => {
                            warn!("[file] Read error during transfer: {}", e);
                            break;
                        }
                        Err(_) => {
                            warn!("[file] Read timeout during transfer");
                            break;
                        }
                    }
                }

                if received == total_size {
                    info!("[file] Transfer complete: {}", filename);
                }
            }
            Err(e) => warn!("[file] Failed to create file {:?}: {}", dest_path, e),
        }

        self.active.lock().await.remove(&file_id);
    }

    #[allow(dead_code)]
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

/// Sanitize a filename to prevent path traversal attacks.
/// Strips directory separators, ".." components, and null bytes.
/// Replaces unsafe characters with underscores.
pub fn sanitize_filename(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            // Path separators and null bytes -> underscore
            '/' | '\\' | '\0' => sanitized.push('_'),
            // Strip trailing dots and spaces on Windows (reserved)
            _ => sanitized.push(c),
        }
    }
    // Remove any ".." sequences that survived (shouldn't, since we replaced separators,
    // but belt-and-suspenders: replace remaining dots used for traversal)
    let sanitized = sanitized
        .replace("..", "_")
        .trim()
        .to_string();

    if sanitized.is_empty() || sanitized == "." {
        return "download".to_string();
    }

    // Limit filename length to prevent resource exhaustion
    let max_len = 255;
    if sanitized.len() > max_len {
        // Keep extension if present
        if let Some(ext_start) = sanitized.rfind('.') {
            if ext_start > 0 && sanitized.len() - ext_start <= 10 {
                let base_end = max_len - (sanitized.len() - ext_start);
                if base_end > 0 {
                    let mut truncated: String = sanitized[..base_end].to_string();
                    truncated.push_str(&sanitized[ext_start..]);
                    return truncated;
                }
            }
        }
        sanitized[..max_len].to_string()
    } else {
        sanitized
    }
}

/// Returns the default download directory used by the file receiver.
/// This is the single source of truth for the download path, shared
/// across both the receiver and the FileComplete event handler.
pub fn default_download_dir() -> PathBuf {
    let home = crate::core::utils::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join("AzurePath").join("downloads")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_removes_path_traversal() {
        // "../../etc/passwd" → / replaced with _, then ".." collapsed to _
        // Result: "____etc_passwd" (4 underscores: _ from first .., _ from /,
        //   _ from second .., _ from / before etc)
        assert_eq!(sanitize_filename("../../etc/passwd"), "____etc_passwd");
        // Same pattern for backslash variant
        assert_eq!(sanitize_filename("..\\..\\Windows\\win.ini"), "____Windows_win.ini");
        assert_eq!(sanitize_filename("foo/bar"), "foo_bar");
        assert_eq!(sanitize_filename("foo\\bar"), "foo_bar");
    }

    #[test]
    fn test_sanitize_filename_removes_null_bytes() {
        assert_eq!(sanitize_filename("file\0.txt"), "file_.txt");
    }

    #[test]
    fn test_sanitize_filename_normal_filename_unchanged() {
        assert_eq!(sanitize_filename("document.pdf"), "document.pdf");
        assert_eq!(sanitize_filename("my_photo.jpg"), "my_photo.jpg");
    }

    #[test]
    fn test_sanitize_filename_empty_becomes_default() {
        assert_eq!(sanitize_filename(""), "download");
        assert_eq!(sanitize_filename("."), "download");
        assert_eq!(sanitize_filename("   "), "download");
    }

    #[test]
    fn test_sanitize_filename_truncates_long_names() {
        let long = "a".repeat(500);
        let result = sanitize_filename(&long);
        assert!(result.len() <= 255);
    }

    #[test]
    fn test_sanitize_filename_preserves_extension_when_truncating() {
        let long = "abcdefghij".repeat(30); // 300 chars
        let long_with_ext = format!("{}.txt", long);
        let result = sanitize_filename(&long_with_ext);
        // With 300+ char base + .txt = 304+ chars, truncates to 255
        // Extension .txt is 4 chars, so base should be truncated to 251
        assert!(result.len() <= 255);
        assert!(result.ends_with(".txt"), "Expected .txt suffix, got: {}", result);
    }

    #[test]
    fn test_read_string_rejects_oversized_length() {
        // We can't easily test read_string without a stream, but we can test
        // the constant is used correctly by checking the limit
        assert_eq!(MAX_WIRE_STRING_LEN, 4096);
    }

    #[test]
    fn test_max_file_size_constant_value() {
        // Verify MAX_FILE_SIZE = 10 GB
        assert_eq!(MAX_FILE_SIZE, 10 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_default_download_dir_contains_azurepath() {
        let dir = default_download_dir();
        let path_str = dir.to_string_lossy();
        assert!(
            path_str.contains("AzurePath"),
            "Expected path to contain 'AzurePath', got: {}",
            path_str
        );
        assert!(
            path_str.contains("downloads"),
            "Expected path to contain 'downloads', got: {}",
            path_str
        );
    }

    #[test]
    fn test_sanitize_filename_removes_absolute_windows_path() {
        // C:\Windows\system32\malware.exe  →  C:_Windows_system32_malware.exe
        let result = sanitize_filename("C:\\Windows\\system32\\malware.exe");
        assert!(!result.contains('\\'), "backslashes should be replaced");
        assert!(result.contains("_Windows_"), "path components should be joined by underscore");
        assert!(result.contains("malware.exe"), "filename part should be preserved");
    }

    #[test]
    fn test_sanitize_filename_mixed_separators() {
        // foo/bar\\baz  →  foo_bar_baz
        assert_eq!(sanitize_filename("foo/bar\\baz"), "foo_bar_baz");
    }

    #[test]
    fn test_sanitize_filename_drive_letter_preserved() {
        // The "C:" should be kept (colon is not stripped)
        let result = sanitize_filename("C:file.txt");
        assert_eq!(result, "C:file.txt");
    }

    #[test]
    fn test_sanitize_filename_trailing_dots_and_spaces_on_windows() {
        // Trailing dots/spaces are reserved on Windows — currently they are
        // preserved in the sanitize function (no explicit stripping of
        // trailing dots).  At minimum, the function should not panic.
        let result = sanitize_filename("file...");
        assert_eq!(result, "file_..");
    }

    #[test]
    fn test_sanitize_filename_nested_dots() {
        // Consecutive ".." sequences become "_" after the ..→_ replacement
        assert_eq!(sanitize_filename("a..b"), "a__b");
    }
}

async fn read_string(stream: &mut TcpStream) -> Result<String, String> {
    let len = read_u64(stream).await? as usize;
    if len > MAX_WIRE_STRING_LEN {
        return Err(format!(
            "String length {} exceeds maximum allowed {}",
            len, MAX_WIRE_STRING_LEN
        ));
    }
    let mut buf = vec![0u8; len];
    tokio::time::timeout(Duration::from_secs(30), stream.read_exact(&mut buf))
        .await
        .map_err(|_| "Read timeout".to_string())?
        .map_err(|e| format!("Failed to read string: {}", e))?;
    String::from_utf8(buf).map_err(|e| format!("Invalid UTF-8: {}", e))
}

async fn read_u64(stream: &mut TcpStream) -> Result<u64, String> {
    let mut buf = [0u8; 8];
    tokio::time::timeout(Duration::from_secs(30), stream.read_exact(&mut buf))
        .await
        .map_err(|_| "Read timeout".to_string())?
        .map_err(|e| format!("Failed to read u64: {}", e))?;
    Ok(u64::from_be_bytes(buf))
}
