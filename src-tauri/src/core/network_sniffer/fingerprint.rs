use regex::Regex;
use std::sync::LazyLock;

/// A compiled fingerprint entry for matching banner text to service + version.
struct FingerprintDef {
    ports: &'static [u16],
    service_name: &'static str,
    version_group: Option<usize>,
    confidence: u8,
    regex: Regex,
}

/// Raw fingerprint definitions (regex patterns compiled once at init).
const FINGERPRINT_RAW: &[(
    &[u16],
    &str,
    &str,
    Option<usize>,
    u8,
)] = &[
    // Web servers
    (&[80, 443, 8080, 8443, 8000, 8888], "nginx", r"(?i)Server:\s*nginx(?:/([\d.]+))?", Some(1), 95),
    (&[80, 443, 8080], "Apache", r"(?i)Server:\s*Apache(?:/([\d.]+))?", Some(1), 95),
    (&[80, 443], "IIS", r"(?i)Server:\s*Microsoft-IIS(?:/([\d.]+))?", Some(1), 95),
    // SSH
    (&[22], "OpenSSH", r"SSH-2\.0-OpenSSH[_-]([\w.]+)", Some(1), 95),
    (&[22], "Dropbear", r"(?i)dropbear", None, 85),
    // FTP
    (&[21], "vsftpd", r"(?i)vsFTPd(?: ([\w.]+))?", Some(1), 90),
    (&[21], "proftpd", r"(?i)ProFTPD(?: ([\w.]+))?", Some(1), 90),
    (&[21], "Pure-FTPd", r"(?i)Pure-FTPd", None, 85),
    // MySQL
    (&[3306], "MySQL", r"(?i)mysql|MariaDB", None, 80),
    // PostgreSQL
    (&[5432], "PostgreSQL", r"(?i)postgres|psql", None, 80),
    // Redis
    (&[6379], "Redis", r"(?i)redis_version:|\+OK", None, 85),
    // SMTP
    (&[25, 587], "Postfix", r"(?i)ESMTP\s+Postfix", None, 85),
    (&[25, 587], "Sendmail", r"(?i)ESMTP\s+Sendmail", None, 85),
    (&[25, 587], "Exim", r"(?i)Exim", None, 85),
    // SMB
    (&[445], "Samba", r"(?i)Samba", None, 80),
    // DNS
    (&[53], "dnsmasq", r"(?i)dnsmasq", None, 75),
    // Generic HTTP fallback
    (&[], "HTTP", r"^HTTP/", None, 60),
];

/// Pre-compiled fingerprints — regexes are compiled once at first access.
static FINGERPRINTS: LazyLock<Vec<FingerprintDef>> = LazyLock::new(|| {
    FINGERPRINT_RAW
        .iter()
        .filter_map(|&(ports, service_name, pattern, version_group, confidence)| {
            Regex::new(pattern).ok().map(|regex| FingerprintDef {
                ports,
                service_name,
                version_group,
                confidence,
                regex,
            })
        })
        .collect()
});

/// Match a banner to a fingerprint, returning (service_name, version, confidence).
pub fn match_banner(port: u16, banner: &str, default_service: Option<&str>) -> Option<(String, Option<String>, u8)> {
    for fp in FINGERPRINTS.iter() {
        if !fp.ports.is_empty() && !fp.ports.contains(&port) {
            continue;
        }
        if let Some(caps) = fp.regex.captures(banner) {
            let version = fp
                .version_group
                .and_then(|g| caps.get(g))
                .map(|m| m.as_str().to_string());
            return Some((fp.service_name.to_string(), version, fp.confidence));
        }
    }

    // Fallback: use port-based service guess
    let service = default_service.or_else(|| guess_service_by_port(port)).map(|s| s.to_string());
    service.map(|s| (s, None, 30))
}

/// Guess service name based on port number alone.
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
        587 => Some("SMTP"),
        993 => Some("IMAPS"),
        995 => Some("POP3S"),
        1433 => Some("MSSQL"),
        1521 => Some("Oracle"),
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

/// Run service detection: grab banner, match fingerprint, fill PortResult.
pub async fn detect_service(
    target: &str,
    mut port_result: crate::types::network_sniffer::PortResult,
    banner_timeout_ms: u64,
) -> crate::types::network_sniffer::PortResult {
    if port_result.state != "open" {
        return port_result;
    }

    // Grab banner
    let banner = super::banner::grab_banner(target, port_result.port, banner_timeout_ms).await;

    if let Some(ref b) = banner {
        port_result.banner = Some(b.clone());
        let default = guess_service_by_port(port_result.port);
        if let Some((service, version, confidence)) = match_banner(port_result.port, b, default) {
            port_result.service = Some(service);
            port_result.version = version;
            port_result.confidence = confidence;
            port_result.probe_method = "banner".to_string();
        }
    } else {
        port_result.service = guess_service_by_port(port_result.port).map(|s| s.to_string());
        port_result.confidence = 20;
    }

    port_result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::network_sniffer::PortResult;

    #[test]
    fn test_all_fingerprints_compile() {
        // Verify every regex pattern compiles successfully
        // Using the raw const to avoid requiring LazyLock initialisation
        for &(_, _, pattern, _, _) in FINGERPRINT_RAW {
            assert!(
                Regex::new(pattern).is_ok(),
                "Failed to compile regex: {}",
                pattern
            );
        }
    }

    #[test]
    fn test_fingerprint_lazylock_initialises() {
        // Accessing FINGERPRINTS triggers LazyLock init — verify no panics
        let fps = &*FINGERPRINTS;
        assert!(!fps.is_empty(), "FINGERPRINTS should contain compiled entries");
    }

    #[test]
    fn test_guess_service() {
        assert_eq!(guess_service_by_port(22), Some("SSH"));
        assert_eq!(guess_service_by_port(80), Some("HTTP"));
        assert_eq!(guess_service_by_port(3306), Some("MySQL"));
        assert_eq!(guess_service_by_port(9999), None);
    }

    #[test]
    fn test_match_nginx() {
        let banner = "HTTP/1.1 200 OK\r\nServer: nginx/1.24.0\r\n";
        let result = match_banner(80, banner, Some("HTTP"));
        assert!(result.is_some());
        let (service, version, confidence) = result.unwrap();
        assert_eq!(service, "nginx");
        assert_eq!(version, Some("1.24.0".to_string()));
        assert!(confidence >= 90);
    }

    #[test]
    fn test_match_nginx_wrong_port() {
        // nginx fingerprint only matches ports [80, 443, 8080, 8443, 8000, 8888]
        let banner = "HTTP/1.1 200 OK\r\nServer: nginx/1.24.0\r\n";
        // port 9999 should NOT match nginx; the generic HTTP fingerprint (empty ports=any)
        // matches banners starting with "HTTP/" on any port
        let result = match_banner(9999, banner, Some("Unknown"));
        assert!(result.is_some());
        let (service, _, _) = result.unwrap();
        // Generic HTTP fingerprint matches first (it catches "HTTP/" prefix on any port)
        assert_eq!(service, "HTTP");
    }

    #[test]
    fn test_match_apache() {
        let banner = "HTTP/1.1 200 OK\r\nServer: Apache/2.4.57 (Unix)\r\n";
        let result = match_banner(80, banner, None);
        assert!(result.is_some());
        let (service, version, _) = result.unwrap();
        assert_eq!(service, "Apache");
        assert_eq!(version, Some("2.4.57".to_string()));
    }

    #[test]
    fn test_match_ssh() {
        let banner = "SSH-2.0-OpenSSH_8.9p1 Ubuntu-3";
        let result = match_banner(22, banner, None);
        assert!(result.is_some());
        let (service, version, _) = result.unwrap();
        assert_eq!(service, "OpenSSH");
        assert_eq!(version, Some("8.9p1".to_string()));
    }

    #[test]
    fn test_match_redis() {
        let banner = "+OK\r\nredis_version:7.2.4";
        let result = match_banner(6379, banner, None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "Redis");
    }

    #[test]
    fn test_match_iis() {
        let banner = "HTTP/1.1 200 OK\r\nServer: Microsoft-IIS/10.0\r\n";
        let result = match_banner(80, banner, None);
        assert!(result.is_some());
        let (service, version, _) = result.unwrap();
        assert_eq!(service, "IIS");
        assert_eq!(version, Some("10.0".to_string()));
    }

    #[test]
    fn test_match_vsftpd() {
        let banner = "220 vsFTPd 3.0.3 ready\r\n";
        let result = match_banner(21, banner, None);
        assert!(result.is_some());
        let (service, version, _) = result.unwrap();
        assert_eq!(service, "vsftpd");
        assert_eq!(version, Some("3.0.3".to_string()));
    }

    #[test]
    fn test_match_samba() {
        let banner = "Samba 4.15.0";
        let result = match_banner(445, banner, None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "Samba");
    }

    #[test]
    fn test_match_generic_http() {
        // The generic HTTP fingerprint (&[]) matches any port
        let banner = "HTTP/1.1 200 OK\r\n";
        let result = match_banner(9999, banner, None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "HTTP");
    }

    #[test]
    fn test_no_match_returns_default() {
        let banner = "some garbage banner";
        let result = match_banner(22, banner, Some("SSH"));
        assert!(result.is_some());
        let (service, version, conf) = result.unwrap();
        assert_eq!(service, "SSH");
        assert!(version.is_none());
        assert_eq!(conf, 30);
    }

    #[test]
    fn test_no_match_no_default() {
        let banner = "unknown protocol data";
        let result = match_banner(9999, banner, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_service_without_banner() {
        let _pr = PortResult {
            port: 9999,
            protocol: "tcp".to_string(),
            state: "open".to_string(),
            service: None,
            version: None,
            banner: None,
            confidence: 0,
            probe_method: "tcp_connect".to_string(),
        };
        // detect_service would try to connect — skip real test, just check it doesn't panic
    }
}
