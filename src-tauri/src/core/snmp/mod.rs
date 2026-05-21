//! SNMP session wrapper using the `snmp2` crate.
//!
//! Provides high-level get/walk operations with error handling.

use std::time::Duration;

use snmp2::SyncSession;
use snmp2::Value;

use crate::types::snmp::SnmpSessionConfig;

pub mod oids;
pub mod scanner;
pub mod collector;
pub mod store;

pub struct SnmpSession {
    session: SyncSession,
}

impl SnmpSession {
    pub fn open(config: &SnmpSessionConfig) -> Result<Self, String> {
        let addr = format!("{}:{}", config.host, config.port);

        let session = SyncSession::new_v2c(
            addr,
            config.community.as_bytes(),
            Some(Duration::from_millis(config.timeout_ms)),
            1,
        )
        .map_err(|e| format!("SNMP session open failed: {e}"))?;

        Ok(Self { session })
    }

    pub fn get(&mut self, oid: &str) -> Result<String, String> {
        let oid_obj: snmp2::Oid = oid
            .parse()
            .map_err(|e| format!("invalid OID {oid}: {e:?}"))?;
        let pdu = self
            .session
            .get(&oid_obj)
            .map_err(|e| format!("SNMP get {oid} failed: {e}"))?;

        for (_, value) in pdu.varbinds {
            return value_to_string(&value)
                .ok_or_else(|| format!("SNMP get {oid}: empty response"));
        }
        Err(format!("SNMP get {oid}: no varbinds in response"))
    }

    pub fn get_multiple(&mut self, oids: &[&str]) -> Result<Vec<Option<String>>, String> {
        let oid_objs: Vec<snmp2::Oid> = oids
            .iter()
            .map(|o| o.parse().map_err(|e| format!("invalid OID {o}: {e:?}")))
            .collect::<Result<Vec<_>, String>>()?;
        let refs: Vec<&snmp2::Oid> = oid_objs.iter().collect();
        let pdu = self
            .session
            .get_many(&refs)
            .map_err(|e| format!("SNMP get multiple failed: {e}"))?;

        let results: Vec<Option<String>> = pdu
            .varbinds
            .into_iter()
            .map(|(_, value)| value_to_string(&value))
            .collect();
        Ok(results)
    }

    pub fn walk(&mut self, oid: &str) -> Result<Vec<(String, String)>, String> {
        let mut current_oid_str: String = oid.to_string();
        let mut results = Vec::new();

        loop {
            let current_oid: snmp2::Oid = current_oid_str
                .parse()
                .map_err(|e| format!("invalid OID {current_oid_str}: {e:?}"))?;
            let pdu = self
                .session
                .getnext(&current_oid)
                .map_err(|e| format!("SNMP walk {oid} failed: {e}"))?;

            // Collect results
            let mut has_varbind = false;
            for (next, value) in pdu.varbinds {
                has_varbind = true;
                let next_str = next.to_string();
                if !next_str.starts_with(oid) {
                    return Ok(results);
                }
                if let Some(s) = value_to_string(&value) {
                    results.push((next_str.clone(), s));
                }
                current_oid_str = next_str;
            }
            if !has_varbind {
                break;
            }
        }

        Ok(results)
    }
}

fn value_to_string(value: &Value<'_>) -> Option<String> {
    match value {
        Value::Integer(i) => Some(i.to_string()),
        Value::OctetString(bytes) => {
            if let Ok(s) = std::str::from_utf8(bytes) {
                Some(s.to_string())
            } else if bytes.len() <= 6 {
                // Likely a MAC address
                Some(
                    bytes
                        .iter()
                        .map(|b| format!("{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(":"),
                )
            } else {
                Some(
                    bytes
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(""),
                )
            }
        }
        Value::ObjectIdentifier(oid) => Some(oid.to_string()),
        Value::IpAddress(ip) => Some(format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])),
        Value::Counter32(n) => Some(n.to_string()),
        Value::Counter64(n) => Some(n.to_string()),
        Value::Unsigned32(n) => Some(n.to_string()),
        Value::Timeticks(n) => Some(n.to_string()),
        Value::Null | Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance => None,
        _ => None,
    }
}
