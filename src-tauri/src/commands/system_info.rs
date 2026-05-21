use tauri::command;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalNetworkInfo {
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
    pub hostname: String,
}

#[command]
pub fn get_local_network_info() -> LocalNetworkInfo {
    let hostname = hostname();
    let (ipv4, ipv6) = get_local_ips();
    LocalNetworkInfo { ipv4, ipv6, hostname }
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".into())
}

fn get_local_ips() -> (Vec<String>, Vec<String>) {
    let mut ipv4 = Vec::new();
    let mut ipv6 = Vec::new();

    if let Ok(ip) = local_ip_address::local_ip() {
        if ip.is_ipv4() {
            ipv4.push(ip.to_string());
        } else {
            ipv6.push(ip.to_string());
        }
    }

    (ipv4, ipv6)
}
