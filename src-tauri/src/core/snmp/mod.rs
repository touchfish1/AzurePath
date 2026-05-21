//! SNMP session wrapper using the `snmp2` crate.
//!
//! Provides high-level get/walk/bulkwalk operations with error handling.

use std::net::SocketAddrV4;
use std::str::FromStr;
use std::time::Duration;

use snmp2::SyncSession;
use snmp2::Value;

use crate::types::snmp::SnmpSessionConfig;

pub struct SnmpSession {
    session: SyncSession,
}

impl SnmpSession {
    pub fn open(config: &SnmpSessionConfig) -> Result<Self, String> {
        let addr = SocketAddrV4::from_str(&format!("{}:{}", config.host, config.port))
            .map_err(|e| format!("invalid SNMP address: {e}"))?;

        let session = SyncSession::new(
            addr,
            config.community.clone(),
            Some(Duration::from_millis(config.timeout_ms)),
            0, // retries
        )
        .map_err(|e| format!("SNMP session open failed: {e}"))?;

        Ok(Self { session })
    }

    pub fn get(&self, oid: &str) -> Result<String, String> {
        let response = self
            .session
            .get(&[oid.into()])
            .map_err(|e| format!("SNMP get {oid} failed: {e}"))?;

        response
            .into_iter()
            .next()
            .and_then(|v| value_to_string(&v))
            .ok_or_else(|| format!("SNMP get {oid}: empty response"))
    }

    pub fn get_multiple(&self, oids: &[&str]) -> Result<Vec<Option<String>>, String> {
        let request: Vec<snmp2::Oid> = oids.iter().map(|o| (*o).into()).collect();
        let response = self
            .session
            .get(&request)
            .map_err(|e| format!("SNMP get multiple failed: {e}"))?;

        Ok(response
            .into_iter()
            .map(|v| value_to_string(&v))
            .collect())
    }

    pub fn walk(&self, oid: &str) -> Result<Vec<(String, String)>, String> {
        let results = self
            .session
            .walk(oid)
            .map_err(|e| format!("SNMP walk {oid} failed: {e}"))?;

        Ok(results
            .into_iter()
            .filter_map(|(oid, value)| {
                value_to_string(&value).map(|s| (oid.to_string(), s))
            })
            .collect())
    }
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Integer(i) => Some(i.to_string()),
        Value::OctetString(bytes) => {
            // Try UTF-8 first, fall back to hex
            String::from_utf8(bytes.clone()).ok()
                .or_else(|| {
                    if bytes.len() <= 6 {
                        // Likely a MAC address
                        Some(bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(":"))
                    } else {
                        Some(bytes.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(""))
                    }
                })
        }
        Value::Oid(oid) => Some(oid.to_string()),
        Value::IpAddress(ip) => Some(ip.to_string()),
        Value::Counter32(n) => Some(n.to_string()),
        Value::Counter64(n) => Some(n.to_string()),
        Value::Gauge32(n) => Some(n.to_string()),
        Value::Timeticks(n) => Some(n.to_string()),
        Value::Null => None,
    }
}
