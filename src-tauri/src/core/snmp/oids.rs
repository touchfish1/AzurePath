//! Well-known SNMP OID constants for system info, interfaces, and monitoring.

/// system
pub const SYS_DESCR: &str = "1.3.6.1.2.1.1.1.0";
pub const SYS_OBJECT_ID: &str = "1.3.6.1.2.1.1.2.0";
pub const SYS_UPTIME: &str = "1.3.6.1.2.1.1.3.0";
pub const SYS_NAME: &str = "1.3.6.1.2.1.1.5.0";

/// interfaces table (ifEntry)
pub const IF_NUMBER: &str = "1.3.6.1.2.1.2.1.0";
pub const IF_TABLE: &str = "1.3.6.1.2.1.2.2.1";
pub const IF_INDEX: &str = "1.3.6.1.2.1.2.2.1.1";
pub const IF_DESCR: &str = "1.3.6.1.2.1.2.2.1.2";
pub const IF_TYPE: &str = "1.3.6.1.2.1.2.2.1.3";
pub const IF_MTU: &str = "1.3.6.1.2.1.2.2.1.4";
pub const IF_SPEED: &str = "1.3.6.1.2.1.2.2.1.5";
pub const IF_PHYS_ADDRESS: &str = "1.3.6.1.2.1.2.2.1.6";
pub const IF_ADMIN_STATUS: &str = "1.3.6.1.2.1.2.2.1.7";
pub const IF_OPER_STATUS: &str = "1.3.6.1.2.1.2.2.1.8";
pub const IF_IN_OCTETS: &str = "1.3.6.1.2.1.2.2.1.10";
pub const IF_OUT_OCTETS: &str = "1.3.6.1.2.1.2.2.1.16";

/// 64-bit interface counters (ifXTable)
pub const IF_HC_IN_OCTETS: &str = "1.3.6.1.2.1.31.1.1.1.6";
pub const IF_HC_OUT_OCTETS: &str = "1.3.6.1.2.1.31.1.1.1.10";

/// IP-MIB
pub const IP_NET_TO_MEDIA_TABLE: &str = "1.3.6.1.2.1.4.22.1";
pub const IP_ROUTE_TABLE: &str = "1.3.6.1.2.1.4.24.2";

/// host resources
pub const HR_PROCESSOR_LOAD: &str = "1.3.6.1.2.1.25.3.3.1.2";
pub const HR_STORAGE_USED: &str = "1.3.6.1.2.1.25.2.3.1.6";
pub const HR_STORAGE_SIZE: &str = "1.3.6.1.2.1.25.2.3.1.5";

/// Common vendor OID prefixes for device type detection
pub const VENDOR_CISCO: &str = "1.3.6.1.4.1.9";
pub const VENDOR_HUAWEI: &str = "1.3.6.1.4.1.2011";
pub const VENDOR_HP: &str = "1.3.6.1.4.1.11";
pub const VENDOR_H3C: &str = "1.3.6.1.4.1.25506";
pub const VENDOR_HIKVISION: &str = "1.3.6.1.4.1.42623";
pub const VENDOR_TP_LINK: &str = "1.3.6.1.4.1.11863";
