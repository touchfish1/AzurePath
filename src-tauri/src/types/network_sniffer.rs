use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnifferOptions {
    pub targets: Vec<String>,
    pub ports: Vec<u16>,
    pub mode: String,          // "fast" | "deep"
    pub concurrency_hosts: u32,
    pub concurrency_ports: u32,
    pub timeout_ms: u64,
    pub probe_services: bool,
}

impl Default for SnifferOptions {
    fn default() -> Self {
        Self {
            targets: vec!["192.168.1.0/24".to_string()],
            ports: vec![
                21, 22, 23, 25, 53, 80, 110, 111, 135, 139, 143, 443, 445,
                993, 995, 1433, 1521, 2049, 3306, 3389, 5432, 5900, 6379,
                8080, 8443, 9090, 27017,
            ],
            mode: "fast".to_string(),
            concurrency_hosts: 10,
            concurrency_ports: 50,
            timeout_ms: 1000,
            probe_services: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceResult {
    pub ip: String,
    pub hostname: Option<String>,
    pub mac: Option<String>,
    pub os: Option<String>,
    pub open_ports: Vec<PortResult>,
    pub is_alive: bool,
    pub scan_mode: String,
    pub scan_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortResult {
    pub port: u16,
    pub protocol: String,
    pub state: String,
    pub service: Option<String>,
    pub version: Option<String>,
    pub banner: Option<String>,
    pub confidence: u8,
    pub probe_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnifferProgress {
    pub total_hosts: u32,
    pub scanned_hosts: u32,
    pub services_found: u32,
    pub current_target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortPreset {
    pub name: String,
    pub label: String,
    pub ports: Vec<u16>,
}

pub fn default_presets() -> Vec<PortPreset> {
    vec![
        PortPreset {
            name: "top100".to_string(),
            label: "常用 (TOP 100)".to_string(),
            ports: vec![
                7, 9, 13, 21, 22, 23, 25, 26, 37, 53, 79, 80, 81, 88, 106,
                110, 111, 113, 119, 135, 139, 143, 144, 179, 199, 389, 427,
                443, 444, 445, 465, 513, 514, 515, 543, 544, 548, 554, 587,
                631, 646, 873, 990, 993, 995, 1025, 1026, 1027, 1028, 1029,
                1110, 1433, 1720, 1723, 1755, 1900, 2000, 2001, 2049, 2121,
                2717, 3000, 3128, 3306, 3389, 3986, 4899, 5000, 5009, 5051,
                5060, 5101, 5190, 5357, 5432, 5631, 5666, 5800, 5900, 6000,
                6001, 6646, 7070, 8000, 8008, 8009, 8080, 8443, 8888, 9000,
                9001, 9090, 9100, 9999, 10000, 32768, 49152, 49153, 49154,
                49155, 49156,
            ],
        },
        PortPreset {
            name: "web".to_string(),
            label: "常见 Web".to_string(),
            ports: vec![80, 443, 8080, 8443, 3000, 5000, 8000, 8888, 9090],
        },
        PortPreset {
            name: "database".to_string(),
            label: "数据库".to_string(),
            ports: vec![3306, 5432, 1433, 1521, 27017, 6379, 9200, 11211],
        },
        PortPreset {
            name: "all".to_string(),
            label: "所有 (1-1024)".to_string(),
            ports: (1..=1024).collect(),
        },
    ]
}
