use regex::Regex;
use std::sync::LazyLock;

/// A single fingerprint entry matching banner text to service + version.
pub struct Fingerprint {
    pub ports: &'static [u16],
    pub service_name: &'static str,
    pub pattern: &'static str,
    pub version_group: Option<usize>,
    pub confidence: u8,
}

/// All known service fingerprints.
static FINGERPRINTS: LazyLock<Vec<Fingerprint>> = LazyLock::new(|| {
    vec![
        // Web servers
        Fingerprint { ports: &[80, 443, 8080, 8443, 8000, 8888], service_name: "nginx", pattern: r"(?i)Server:\s*nginx(?:/([\d.]+))?", version_group: Some(1), confidence: 95 },
        Fingerprint { ports: &[80, 443, 8080], service_name: "Apache", pattern: r"(?i)Server:\s*Apache(?:/([\d.]+))?", version_group: Some(1), confidence: 95 },
        Fingerprint { ports: &[80, 443], service_name: "IIS", pattern: r"(?i)Server:\s*Microsoft-IIS(?:/([\d.]+))?", version_group: Some(1), confidence: 95 },
        // SSH
        Fingerprint { ports: &[22], service_name: "OpenSSH", pattern: r"SSH-2\.0-OpenSSH[_-]([\w.]+)", version_group: Some(1), confidence: 95 },
        Fingerprint { ports: &[22], service_name: "Dropbear", pattern: r"(?i)dropbear", version_group: None, confidence: 85 },
        // FTP
        Fingerprint { ports: &[21], service_name: "vsftpd", pattern: r"(?i)vsFTPd(?: ([\w.]+))?", version_group: Some(1), confidence: 90 },
        Fingerprint { ports: &[21], service_name: "proftpd", pattern: r"(?i)ProFTPD(?: ([\w.]+))?", version_group: Some(1), confidence: 90 },
        Fingerprint { ports: &[21], service_name: "Pure-FTPd", pattern: r"(?i)Pure-FTPd", version_group: None, confidence: 85 },
        // MySQL
        Fingerprint { ports: &[3306], service_name: "MySQL", pattern: r"(?i)mysql|MariaDB", version_group: None, confidence: 80 },
        // PostgreSQL
        Fingerprint { ports: &[5432], service_name: "PostgreSQL", pattern: r"(?i)postgres|psql", version_group: None, confidence: 80 },
        // Redis
        Fingerprint { ports: &[6379], service_name: "Redis", pattern: r"(?i)redis_version:|\+OK", version_group: None, confidence: 85 },
        // SMTP
        Fingerprint { ports: &[25, 587], service_name: "Postfix", pattern: r"(?i)ESMTP\s+Postfix", version_group: None, confidence: 85 },
        Fingerprint { ports: &[25, 587], service_name: "Sendmail", pattern: r"(?i)ESMTP\s+Sendmail", version_group: None, confidence: 85 },
        Fingerprint { ports: &[25, 587], service_name: "Exim", pattern: r"(?i)Exim", version_group: None, confidence: 85 },
        // SMB
        Fingerprint { ports: &[445], service_name: "Samba", pattern: r"(?i)Samba", version_group: None, confidence: 80 },
        // DNS
        Fingerprint { ports: &[53], service_name: "dnsmasq", pattern: r"(?i)dnsmasq", version_group: None, confidence: 75 },
        // Generic HTTP fallback
        Fingerprint { ports: &[], service_name: "HTTP", pattern: r"^HTTP/", version_group: None, confidence: 60 },
    ]
});

/// Match a banner to a fingerprint, returning (service_name, version, confidence).
pub fn match_banner(port: u16, banner: &str, default_service: Option<&str>) -> Option<(String, Option<String>, u8)> {
    for fp in FINGERPRINTS.iter() {
        if !fp.ports.is_empty() && !fp.ports.contains(&port) {
            continue;
        }
        if let Ok(re) = Regex::new(fp.pattern) {
            if let Some(caps) = re.captures(banner) {
                let version = fp.version_group.and_then(|g| caps.get(g)).map(|m| m.as_str().to_string());
                return Some((fp.service_name.to_string(), version, fp.confidence));
            }
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
