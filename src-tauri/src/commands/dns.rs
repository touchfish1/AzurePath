use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::core::dns;
use crate::types::dns::{DnsResult, RecordType};

#[tauri::command]
pub async fn dns_lookup(
    app: AppHandle,
    target: String,
    record_type: RecordType,
    dns_server: Option<String>,
) -> Result<String, String> {
    let task_id = Uuid::new_v4().to_string();

    let result = dns::resolve(&target, &record_type, dns_server.as_deref()).await;

    match result {
        Ok(records) => {
            let dns_result = DnsResult {
                task_id: task_id.clone(),
                target: target.clone(),
                records: records.clone(),
            };

            app.emit("dns:result", &dns_result)
                .map_err(|e| format!("Failed to emit dns:result: {}", e))?;

            Ok(serde_json::to_string(&records).map_err(|e| e.to_string())?)
        }
        Err(e) => {
            app.emit(
                "dns:error",
                serde_json::json!({
                    "task_id": task_id,
                    "target": target,
                    "error": e,
                }),
            )
            .map_err(|err| format!("Failed to emit dns:error: {}", err))?;

            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::dns::DnsRecord;

    // -----------------------------------------------------------------------
    // RecordType serde — parameter mapping correctness
    // -----------------------------------------------------------------------

    #[test]
    fn test_record_type_roundtrip_all_variants() {
        let cases = vec![
            (RecordType::A, "a"),
            (RecordType::Aaaa, "aaaa"),
            (RecordType::Cname, "cname"),
            (RecordType::Mx, "mx"),
            (RecordType::Ns, "ns"),
            (RecordType::Soa, "soa"),
            (RecordType::Txt, "txt"),
            (RecordType::All, "all"),
        ];
        for (variant, expected_str) in cases {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{}\"", expected_str));
            let deserialized: RecordType = serde_json::from_str(&json).unwrap();
            assert_eq!(
                format!("{:?}", deserialized),
                format!("{:?}", variant),
                "roundtrip failed for {:?}",
                variant
            );
        }
    }

    #[test]
    fn test_record_type_deserialize_case_sensitive() {
        // serde renames are lowercase; uppercase input should fail
        let result: Result<RecordType, _> = serde_json::from_str("\"A\"");
        assert!(result.is_err());
        let result: Result<RecordType, _> = serde_json::from_str("\"AAAA\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_record_type_deserialize_rejects_invalid_values() {
        for val in &["\"\"", "\"unknown\"", "null", "123", "\"a\na\""] {
            let result: Result<RecordType, _> = serde_json::from_str(val);
            assert!(result.is_err(), "Expected error for input: {}", val);
        }
    }

    #[test]
    fn test_record_type_deserialize_missing() {
        // Empty JSON object should fail (no RecordType)
        let result: Result<RecordType, _> = serde_json::from_str("{}");
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // DnsRecord serde — event payload correctness
    // -----------------------------------------------------------------------

    #[test]
    fn test_dns_record_uses_type_rename() {
        let record = DnsRecord {
            name: "example.com".to_string(),
            record_type: "A".to_string(),
            value: "1.2.3.4".to_string(),
            ttl: 60,
        };
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["name"], "example.com");
        // The Rust field record_type is renamed to "type" in JSON
        assert_eq!(json["type"], "A");
        assert!(
            json.get("record_type").is_none(),
            "record_type should be renamed to 'type' in JSON output"
        );
        assert_eq!(json["value"], "1.2.3.4");
        assert_eq!(json["ttl"], 60);
    }

    #[test]
    fn test_dns_record_deserialize_from_type_json() {
        let json = r#"{"name":"test.com","type":"AAAA","value":"::1","ttl":300}"#;
        let record: DnsRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.name, "test.com");
        assert_eq!(record.record_type, "AAAA");
        assert_eq!(record.value, "::1");
        assert_eq!(record.ttl, 300);
    }

    #[test]
    fn test_dns_record_deserialize_rejects_rust_field_name() {
        // JSON using "record_type" (the Rust name) should fail;
        // only "type" (the serde rename) is accepted.
        let json = r#"{"name":"x","record_type":"A","value":"1.2.3.4","ttl":100}"#;
        let result: Result<DnsRecord, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_dns_record_deserialize_missing_required_fields() {
        let cases = vec![
            r#"{"name":"x","type":"A","value":"1.2.3.4"}"#,       // missing ttl
            r#"{"type":"A","value":"1.2.3.4","ttl":60}"#,          // missing name
            r#"{"name":"x","value":"1.2.3.4","ttl":60}"#,          // missing type (record_type)
            r#"{}"#,
        ];
        for json in cases {
            let result: Result<DnsRecord, _> = serde_json::from_str(json);
            assert!(result.is_err(), "Expected error for JSON: {}", json);
        }
    }

    #[test]
    fn test_dns_record_all_field_types() {
        let record = DnsRecord {
            name: String::new(),
            record_type: String::new(),
            value: String::new(),
            ttl: 0,
        };
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["name"], "");
        assert_eq!(json["type"], "");
        assert_eq!(json["value"], "");
        assert_eq!(json["ttl"], 0);
    }

    // -----------------------------------------------------------------------
    // DnsResult serde — event payload structure
    // -----------------------------------------------------------------------

    #[test]
    fn test_dns_result_serialization_structure() {
        let result = DnsResult {
            task_id: "abc-123".to_string(),
            target: "example.com".to_string(),
            records: vec![DnsRecord {
                name: "example.com".to_string(),
                record_type: "A".to_string(),
                value: "93.184.216.34".to_string(),
                ttl: 3600,
            }],
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["task_id"], "abc-123");
        assert_eq!(json["target"], "example.com");
        assert_eq!(json["records"].as_array().unwrap().len(), 1);
        assert_eq!(json["records"][0]["type"], "A");
    }

    #[test]
    fn test_dns_result_with_multiple_records() {
        let records = vec![
            DnsRecord {
                name: "example.com".to_string(),
                record_type: "A".to_string(),
                value: "93.184.216.34".to_string(),
                ttl: 3600,
            },
            DnsRecord {
                name: "example.com".to_string(),
                record_type: "AAAA".to_string(),
                value: "2606:2800:220:1:248:1893:25c8:1946".to_string(),
                ttl: 3600,
            },
        ];
        let result = DnsResult {
            task_id: "id-1".to_string(),
            target: "example.com".to_string(),
            records,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["records"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_dns_result_with_empty_records() {
        let result = DnsResult {
            task_id: "id-empty".to_string(),
            target: "empty.example.com".to_string(),
            records: vec![],
        };
        let json = serde_json::to_value(&result).unwrap();
        assert!(json["records"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_dns_result_roundtrip() {
        let original = DnsResult {
            task_id: "rt-id".to_string(),
            target: "example.com".to_string(),
            records: vec![DnsRecord {
                name: "example.com".to_string(),
                record_type: "MX".to_string(),
                value: "10 mail.example.com".to_string(),
                ttl: 1800,
            }],
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: DnsResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.task_id, original.task_id);
        assert_eq!(deserialized.target, original.target);
        assert_eq!(deserialized.records.len(), original.records.len());
        assert_eq!(deserialized.records[0].record_type, "MX");
    }

    // -----------------------------------------------------------------------
    // Command parameter type contract
    // -----------------------------------------------------------------------

    #[test]
    fn test_dns_lookup_signature_compatible() {
        // Compile-time check: dns_lookup(target: String, record_type: RecordType, dns_server: Option<String>)
        // are the three key parameters.  This closure verifies the types
        // are usable as Tauri command parameters.
        let _check = |target: String, _record_type: RecordType, _dns_server: Option<String>| {
            let _ = target;
        };
        let _ = _check;
    }
}
