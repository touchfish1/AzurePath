use std::path::PathBuf;

#[cfg(target_os = "windows")]
use encoding_rs::GBK;

/// Get the user's home directory.
/// Checks `USERPROFILE` (Windows) first, then `HOME` (Unix).
pub fn home_dir() -> Option<PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(PathBuf::from)
}

/// Decode process output bytes to a UTF-8 string.
/// On Windows, falls back to GBK decoding if the input is not valid UTF-8.
/// On non-Windows platforms, uses lossy UTF-8 conversion.
pub fn decode_output(bytes: &[u8]) -> String {
    #[cfg(target_os = "windows")]
    {
        String::from_utf8(bytes.to_vec())
            .unwrap_or_else(|_| GBK.decode(bytes).0.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        String::from_utf8_lossy(bytes).to_string()
    }
}

/// Guess the service name based on port number.
///
/// Merges mappings from `port_scan::guess_service()` and
/// `network_sniffer::fingerprint::guess_service_by_port()`.
/// When names conflict, the more detailed name is used.
pub fn guess_service_by_port(port: u16) -> Option<&'static str> {
    match port {
        21 => Some("FTP"),
        22 => Some("SSH"),
        23 => Some("Telnet"),
        25 => Some("SMTP"),
        53 => Some("DNS"),
        80 => Some("HTTP"),
        110 => Some("POP3"),
        111 => Some("RPC"),
        135 => Some("RPC"),
        139 => Some("NetBIOS"),
        143 => Some("IMAP"),
        443 => Some("HTTPS"),
        445 => Some("SMB"),
        465 => Some("SMTPS"),
        587 => Some("SMTP Submission"),
        993 => Some("IMAPS"),
        995 => Some("POP3S"),
        1433 => Some("MSSQL"),
        1521 => Some("Oracle DB"),
        2049 => Some("NFS"),
        3306 => Some("MySQL"),
        3389 => Some("RDP"),
        5432 => Some("PostgreSQL"),
        5900 => Some("VNC"),
        6379 => Some("Redis"),
        8080 => Some("HTTP-Alt"),
        8443 => Some("HTTPS-Alt"),
        9090 => Some("HTTP-Alt"),
        27017 => Some("MongoDB"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_dir_returns_some() {
        let dir = home_dir();
        assert!(dir.is_some(), "Expected home_dir() to return a path");
        let path = dir.unwrap();
        assert!(path.is_absolute(), "Home directory should be absolute");
    }

    #[test]
    fn test_decode_output_valid_utf8() {
        let decoded = decode_output(b"Reply from 8.8.8.8: bytes=32 time=11ms TTL=118");
        assert_eq!(decoded, "Reply from 8.8.8.8: bytes=32 time=11ms TTL=118");
    }

    #[test]
    fn test_decode_output_empty() {
        let decoded = decode_output(b"");
        assert_eq!(decoded, "");
    }

    #[test]
    fn test_decode_output_invalid_utf8_does_not_panic() {
        decode_output(b"\xff\xfe\x00\x01");
    }

    #[test]
    fn test_guess_service_known_ports() {
        let cases = [
            (21, "FTP"),
            (22, "SSH"),
            (23, "Telnet"),
            (25, "SMTP"),
            (53, "DNS"),
            (80, "HTTP"),
            (110, "POP3"),
            (111, "RPC"),
            (135, "RPC"),
            (139, "NetBIOS"),
            (143, "IMAP"),
            (443, "HTTPS"),
            (445, "SMB"),
            (465, "SMTPS"),
            (587, "SMTP Submission"),
            (993, "IMAPS"),
            (995, "POP3S"),
            (1433, "MSSQL"),
            (1521, "Oracle DB"),
            (2049, "NFS"),
            (3306, "MySQL"),
            (3389, "RDP"),
            (5432, "PostgreSQL"),
            (5900, "VNC"),
            (6379, "Redis"),
            (8080, "HTTP-Alt"),
            (8443, "HTTPS-Alt"),
            (9090, "HTTP-Alt"),
            (27017, "MongoDB"),
        ];
        for (port, expected) in &cases {
            assert_eq!(guess_service_by_port(*port), Some(*expected));
        }
    }

    #[test]
    fn test_guess_service_unknown_ports() {
        assert_eq!(guess_service_by_port(0), None);
        assert_eq!(guess_service_by_port(9999), None);
        assert_eq!(guess_service_by_port(65535), None);
    }

    #[test]
    fn test_decode_output_gbk_bytes_nonempty() {
        // GBK-encoded bytes (Chinese characters "你好" in GBK).
        // On Windows this decodes through the GBK fallback path;
        // on Unix the lossy UTF-8 path is used.  Either way the
        // result must be non-empty and must not panic.
        let gbk = [0xc4, 0xe3, 0xba, 0xc3];
        let result = decode_output(&gbk);
        assert!(!result.is_empty(), "GBK bytes should produce some output");
    }

    #[test]
    fn test_decode_output_utf8_chinese_chars() {
        // Valid UTF-8 Chinese characters — must decode cleanly on all platforms.
        let utf8 = "你好世界".as_bytes();
        let result = decode_output(utf8);
        assert_eq!(result, "你好世界");
    }
}
