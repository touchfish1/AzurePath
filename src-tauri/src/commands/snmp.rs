//! SNMP Tauri commands — device discovery, interface query, data collection.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::core::snmp::collector::SnmpCollector;
use crate::core::snmp::oids;
use crate::core::snmp::scanner::SnmpScanner;
use crate::core::snmp::store::SnmpStore;
use crate::core::snmp::SnmpSession;
use crate::types::snmp::{
    SnmpArpEntry, SnmpDevice, SnmpInterface, SnmpSample, SnmpSessionConfig,
};

static STORE: OnceLock<Arc<SnmpStore>> = OnceLock::new();

fn store() -> &'static Arc<SnmpStore> {
    STORE.get().expect("SnmpStore not initialized")
}

pub async fn snmp_init() -> Result<(), String> {
    let s = SnmpStore::new()?;
    STORE.get_or_init(|| Arc::new(s));
    Ok(())
}

#[tauri::command]
pub async fn snmp_discover(
    app: AppHandle,
    cidr: String,
    community: String,
) -> Result<Vec<SnmpDevice>, String> {
    let config = SnmpSessionConfig {
        host: "0.0.0.0".into(),
        port: 161,
        community,
        timeout_ms: 2000,
    };

    let scanner = SnmpScanner::new(config.clone());
    let app_clone = app.clone();

    let devices = scanner
        .scan(&cidr, move |progress| {
            let _ = app_clone.emit("snmp:progress", &progress);
        })
        .await?;

    // Save discovered devices
    for device in &devices {
        store().upsert_device(device)?;
    }

    let _ = app.emit("snmp:discover-complete", &devices);
    Ok(devices)
}

#[tauri::command]
pub async fn snmp_list_devices() -> Result<Vec<SnmpDevice>, String> {
    store().list_devices()
}

#[tauri::command]
pub async fn snmp_delete_device(id: String) -> Result<(), String> {
    store().delete_device(&id)
}

#[tauri::command]
pub async fn snmp_get_interfaces(
    host: String,
    community: String,
) -> Result<Vec<SnmpInterface>, String> {
    let config = SnmpSessionConfig {
        host,
        port: 161,
        community,
        timeout_ms: 3000,
    };
    let mut session = SnmpSession::open(&config)?;

    let entries = session.walk(oids::IF_TABLE)?;
    let mut interfaces: HashMap<u32, SnmpInterface> = HashMap::new();

    for (oid, value) in &entries {
        let parts: Vec<&str> = oid.rsplitn(2, '.').collect();
        let index: u32 = parts[0].parse().unwrap_or(0);
        if index == 0 {
            continue;
        }

        let iface = interfaces.entry(index).or_insert(SnmpInterface {
            index,
            name: String::new(),
            description: String::new(),
            mac: String::new(),
            ip: String::new(),
            speed: 0,
            admin_status: 0,
            oper_status: 0,
        });

        if oid.starts_with(oids::IF_DESCR) {
            iface.description = value.clone();
            iface.name = value.clone();
        } else if oid.starts_with(oids::IF_PHYS_ADDRESS) && value != "00:00:00:00:00:00" {
            iface.mac = value.clone();
        } else if oid.starts_with(oids::IF_SPEED) {
            iface.speed = value.parse().unwrap_or(0);
        } else if oid.starts_with(oids::IF_ADMIN_STATUS) {
            iface.admin_status = value.parse().unwrap_or(0);
        } else if oid.starts_with(oids::IF_OPER_STATUS) {
            iface.oper_status = value.parse().unwrap_or(0);
        }
    }

    let mut result: Vec<SnmpInterface> = interfaces.into_values().collect();
    result.sort_by_key(|i| i.index);
    Ok(result)
}

#[tauri::command]
pub async fn snmp_get_arp_table(
    host: String,
    community: String,
) -> Result<Vec<SnmpArpEntry>, String> {
    let config = SnmpSessionConfig {
        host,
        port: 161,
        community,
        timeout_ms: 3000,
    };
    let mut session = SnmpSession::open(&config)?;
    let entries = session.walk("1.3.6.1.2.1.4.22.1")?;

    let mut arp_entries: HashMap<String, (String, String)> = HashMap::new();

    for (oid, value) in &entries {
        let parts: Vec<&str> = oid.rsplitn(2, '.').collect();
        let key = parts[0].to_string();

        let entry = arp_entries.entry(key).or_default();
        if oid.starts_with("1.3.6.1.2.1.4.22.1.2") {
            entry.1 = value.clone(); // MAC
        } else if oid.starts_with("1.3.6.1.2.1.4.22.1.3") {
            entry.0 = value.clone(); // IP
        }
    }

    let mut parsed_entries: Vec<SnmpArpEntry> = arp_entries
        .into_values()
        .filter(|(ip, _)| !ip.is_empty())
        .take(200)
        .map(|(ip, mac)| SnmpArpEntry {
            ip,
            mac,
            interface: String::new(),
        })
        .collect();

    parsed_entries.sort_by(|a, b| {
        a.ip.split('.')
            .filter_map(|p| p.parse::<u32>().ok())
            .cmp(b.ip.split('.').filter_map(|p| p.parse::<u32>().ok()))
    });

    Ok(parsed_entries)
}

#[tauri::command]
pub async fn snmp_get_history(
    device_id: String,
    limit: Option<u32>,
) -> Result<Vec<SnmpSample>, String> {
    store().get_history(&device_id, limit.unwrap_or(300))
}

// Global collector instances
static COLLECTORS: OnceLock<Arc<Mutex<HashMap<String, SnmpCollector>>>> = OnceLock::new();

fn collectors() -> &'static Arc<Mutex<HashMap<String, SnmpCollector>>> {
    COLLECTORS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

#[tauri::command]
pub async fn snmp_start_collect(
    app: AppHandle,
    host: String,
    community: String,
    interval_secs: Option<u64>,
) -> Result<(), String> {
    let config = SnmpSessionConfig {
        host: host.clone(),
        port: 161,
        community,
        timeout_ms: 3000,
    };

    let collector = SnmpCollector::new();
    let app_clone = app.clone();

    collector
        .start(config, interval_secs.unwrap_or(10), move |sample| {
            let _ = app_clone.emit("snmp:sample", &sample);
        })
        .await?;

    collectors()
        .lock()
        .await
        .insert(host, collector);

    Ok(())
}

#[tauri::command]
pub async fn snmp_stop_collect(host: String) -> Result<(), String> {
    let mut map = collectors().lock().await;
    if let Some(collector) = map.remove(&host) {
        collector.stop().await;
    }
    Ok(())
}
