//! SQLite persistence for SNMP devices and collected data.

use rusqlite::{params, Connection};
use std::sync::Mutex;

use crate::types::snmp::SnmpDevice;
use crate::types::snmp::SnmpSample;

pub struct SnmpStore {
    conn: Mutex<Connection>,
}

impl SnmpStore {
    pub fn new() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| e.to_string())?;
        let store = Self { conn: Mutex::new(conn) };
        store.init_tables()?;
        Ok(store)
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS snmp_devices (
                id TEXT PRIMARY KEY,
                ip TEXT NOT NULL,
                hostname TEXT,
                sys_descr TEXT,
                sys_object_id TEXT,
                vendor TEXT,
                model TEXT,
                uptime INTEGER DEFAULT 0,
                community TEXT,
                last_seen TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS snmp_samples (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                if_index INTEGER NOT NULL,
                in_bps REAL DEFAULT 0,
                out_bps REAL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_snmp_samples_device ON snmp_samples(device_id, timestamp);
            ",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn upsert_device(&self, device: &SnmpDevice) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO snmp_devices (id, ip, hostname, sys_descr, sys_object_id, vendor, model, uptime, community, last_seen) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                device.id, device.ip, device.hostname, device.sys_descr,
                device.sys_object_id, device.vendor, device.model,
                device.uptime, device.community, device.last_seen
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_devices(&self) -> Result<Vec<SnmpDevice>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, ip, hostname, sys_descr, sys_object_id, vendor, model, uptime, community, last_seen \
                 FROM snmp_devices ORDER BY ip"
            )
            .map_err(|e| e.to_string())?;
        let devices = stmt
            .query_map([], |row| {
                Ok(SnmpDevice {
                    id: row.get(0)?,
                    ip: row.get(1)?,
                    hostname: row.get(2)?,
                    sys_descr: row.get(3)?,
                    sys_object_id: row.get(4)?,
                    vendor: row.get(5)?,
                    model: row.get(6)?,
                    uptime: row.get(7)?,
                    community: row.get(8)?,
                    last_seen: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(devices)
    }

    pub fn delete_device(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM snmp_samples WHERE device_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM snmp_devices WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_sample(
        &self,
        device_id: &str,
        timestamp: &str,
        if_index: u32,
        in_bps: f64,
        out_bps: f64,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO snmp_samples (device_id, timestamp, if_index, in_bps, out_bps) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![device_id, timestamp, if_index, in_bps, out_bps],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_history(
        &self,
        device_id: &str,
        limit: u32,
    ) -> Result<Vec<SnmpSample>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT device_id, timestamp, if_index, in_bps, out_bps FROM snmp_samples \
                 WHERE device_id = ?1 ORDER BY timestamp DESC LIMIT ?2"
            )
            .map_err(|e| e.to_string())?;
        let samples = stmt
            .query_map(params![device_id, limit], |row| {
                Ok(SnmpSample {
                    device_id: row.get(0)?,
                    timestamp: row.get(1)?,
                    if_index: row.get(2)?,
                    in_bps: row.get(3)?,
                    out_bps: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(samples)
    }
}
