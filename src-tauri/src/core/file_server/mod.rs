use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

/// Wrapper that recovers from a poisoned mutex by locking a new Mutex, effectively
/// restarting the shared state. Used to prevent a single panicking handler from
/// permanently disabling the file server.
fn lock_map(map: &Mutex<HashMap<String, String>>) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
    match map.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!("[file_server] Mutex was poisoned, recovering");
            poisoned.into_inner()
        }
    }
}

#[derive(Clone)]
pub struct FileServerHandle {
    port: u16,
    files: Arc<Mutex<HashMap<String, String>>>,
}

/// Minimal HTTP file server for serving completed file transfers.
/// Listens on 127.0.0.1:0 (localhost, dynamic port) and serves files
/// via GET /download/{file_id}/{filename}.
pub struct FileServer {
    handle: FileServerHandle,
    _thread: std::thread::JoinHandle<()>,
}

impl FileServer {
    pub fn new() -> Result<Self, String> {
        let listener =
            TcpListener::bind("127.0.0.1:0").map_err(|e| format!("Failed to bind: {}", e))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("Failed to get port: {}", e))?
            .port();
        let files: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
        let files_clone = files.clone();

        info!("[file_server] Starting on 127.0.0.1:{}", port);

        let thread = std::thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => handle_request(stream, &files_clone),
                    Err(e) => {
                        warn!("[file_server] Accept error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(FileServer {
            handle: FileServerHandle { port, files },
            _thread: thread,
        })
    }

    pub fn handle(&self) -> &FileServerHandle {
        &self.handle
    }
}

impl FileServerHandle {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn register_file(&self, file_id: &str, file_path: &str) {
        let mut map = lock_map(&self.files);
        map.insert(file_id.to_string(), file_path.to_string());
    }

    pub fn get_path(&self, file_id: &str) -> Option<String> {
        let map = lock_map(&self.files);
        map.get(file_id).cloned()
    }

    pub fn download_url(&self, file_id: &str, filename: &str) -> String {
        format!(
            "http://127.0.0.1:{}/download/{}/{}",
            self.port,
            file_id,
            url_encode(filename)
        )
    }
}

fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push_str("%20"),
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

fn handle_request(mut stream: TcpStream, files: &Arc<Mutex<HashMap<String, String>>>) {
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }

    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 || parts[0] != "GET" {
        send_response(&mut stream, 405, "Method Not Allowed", b"Method not allowed");
        return;
    }

    let path = parts[1];

    // Parse /download/{file_id}/{filename}
    let path_parts: Vec<&str> = path.split('/').collect();
    if path_parts.len() < 4 || path_parts[1] != "download" {
        send_response(&mut stream, 404, "Not Found", b"Not found");
        return;
    }

    let file_id = path_parts[2];

    let file_path = {
        let map = lock_map(files);
        map.get(file_id).cloned()
    };

    match file_path {
        Some(path) => serve_file(&mut stream, &path),
        None => send_response(&mut stream, 404, "Not Found", b"File not found"),
    }
}

fn serve_file(stream: &mut TcpStream, file_path: &str) {
    let path = Path::new(file_path);
    if !path.exists() {
        send_response(stream, 404, "Not Found", b"File not found on disk");
        return;
    }

    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => {
            send_response(stream, 500, "Internal Server Error", b"Failed to read metadata");
            return;
        }
    };

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            send_response(stream, 500, "Internal Server Error", b"Failed to open file");
            return;
        }
    };

    let size = metadata.len();
    let filename_raw = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download");

    // SECURITY: Sanitize filename for HTTP headers to prevent injection
    // (e.g., CRLF injection, quote injection in Content-Disposition).
    let filename = sanitize_header_filename(filename_raw);

    let response_headers = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/octet-stream\r\n\
         Content-Disposition: attachment; filename=\"{}\"\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         Access-Control-Allow-Origin: *\r\n\
         \r\n",
        filename, size
    );

    if stream.write_all(response_headers.as_bytes()).is_err() {
        return;
    }

    let mut buffer = [0u8; 65536];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                if stream.write_all(&buffer[..n]).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

fn send_response(stream: &mut TcpStream, status: u16, reason: &str, body: &[u8]) {
    let response = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Length: {}\r\n\
         Content-Type: text/plain\r\n\
         Connection: close\r\n\
         Access-Control-Allow-Origin: *\r\n\
         \r\n",
        status,
        reason,
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(body);
}

/// Sanitize a filename for use in HTTP Content-Disposition headers.
/// Strips characters that could enable header injection (CR, LF, quotes).
fn sanitize_header_filename(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            '\r' | '\n' | '"' | '\\' => result.push('_'),
            _ => result.push(c),
        }
    }
    if result.is_empty() {
        result = "download".to_string();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_header_filename_removes_injection_chars() {
        // \r and \n both become _, dot is preserved
        assert_eq!(sanitize_header_filename("file\r\n.txt"), "file__.txt");
        assert_eq!(sanitize_header_filename("file\".txt"), "file_.txt");
        assert_eq!(sanitize_header_filename("file\\.txt"), "file_.txt");
    }

    #[test]
    fn test_sanitize_header_filename_normal() {
        assert_eq!(sanitize_header_filename("document.pdf"), "document.pdf");
        assert_eq!(sanitize_header_filename("my file.txt"), "my file.txt");
    }

    #[test]
    fn test_sanitize_header_filename_empty_becomes_default() {
        assert_eq!(sanitize_header_filename(""), "download");
    }

    #[test]
    fn test_url_encode_special_chars() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("foo/bar"), "foo%2Fbar");
        assert_eq!(url_encode("normal.txt"), "normal.txt");
    }

    #[test]
    fn test_lock_map_recovers_from_poison() {
        let map = Arc::new(Mutex::new(HashMap::new()));
        // Poison the lock
        let map_clone = map.clone();
        let _ = std::thread::spawn(move || {
            let _guard = map_clone.lock().unwrap();
            panic!("intentional panic to poison mutex");
        })
        .join();

        // lock_map should recover, not panic
        let guard = lock_map(&map);
        assert!(guard.is_empty());
    }
}
