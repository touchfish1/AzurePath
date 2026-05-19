//! Native ICMP Echo ping via raw sockets.
//!
//! ## Unix
//! Creates a raw ICMP socket via `libc` and sends Echo Request (type=8)
//! packets with RFC 1071 checksum, then parses Echo Reply (type=0).
//! Requires root (CAP_NET_RAW on Linux).
//!
//! ## Windows
//! Raw ICMP sockets require Administrator privileges.  This module returns
//! a clear error on permission failure.  The system `ping.exe` (used in
//! the parent module) works without elevation and serves as the fallback.

use std::net::IpAddr;
use std::time::{Duration, Instant};

use super::PingResult;

// ── Unix: raw ICMP socket via libc ─────────────────────────────────────

#[cfg(unix)]
mod imp {
    use libc;
    use std::io;
    use std::mem;
    use std::net::SocketAddrV4;
    use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};
    use std::time::Duration;

    pub struct RawIcmpSocket {
        fd: OwnedFd,
    }

    impl std::fmt::Debug for RawIcmpSocket {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("RawIcmpSocket")
                .field("fd", &self.fd.as_raw_fd())
                .finish()
        }
    }

    impl RawIcmpSocket {
        pub fn new() -> io::Result<Self> {
            let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, libc::IPPROTO_ICMP) };
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self {
                fd: unsafe { OwnedFd::from_raw_fd(fd) },
            })
        }

        pub fn set_read_timeout(&self, timeout: Duration) -> io::Result<()> {
            let tv = libc::timeval {
                tv_sec: timeout.as_secs() as libc::time_t,
                tv_usec: timeout.subsec_micros() as libc::suseconds_t,
            };
            let ret = unsafe {
                libc::setsockopt(
                    self.fd.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_RCVTIMEO,
                    &tv as *const _ as *const libc::c_void,
                    mem::size_of::<libc::timeval>() as libc::socklen_t,
                )
            };
            if ret != 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }

        pub fn send_icmp(&self, dest: &SocketAddrV4, packet: &[u8]) -> io::Result<usize> {
            let addr = libc::sockaddr_in {
                sin_family: libc::AF_INET as libc::sa_family_t,
                sin_port: 0,
                sin_addr: libc::in_addr {
                    s_addr: u32::from(*dest.ip()).to_be(),
                },
                sin_zero: [0u8; 8],
            };
            let ret = unsafe {
                libc::sendto(
                    self.fd.as_raw_fd(),
                    packet.as_ptr() as *const libc::c_void,
                    packet.len(),
                    0,
                    &addr as *const _ as *const libc::sockaddr,
                    mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
                )
            };
            if ret < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(ret as usize)
            }
        }

        pub fn recv_icmp(&self, buf: &mut [u8]) -> io::Result<usize> {
            let ret = unsafe {
                libc::recvfrom(
                    self.fd.as_raw_fd(),
                    buf.as_mut_ptr() as *mut libc::c_void,
                    buf.len(),
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if ret < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(ret as usize)
            }
        }
    }
}

// ── Windows: stub — returns permission error ──────────────────────────

#[cfg(windows)]
mod imp {
    use std::io;
    use std::net::SocketAddrV4;
    use std::time::Duration;

    #[derive(Debug)]
    pub struct RawIcmpSocket;

    impl RawIcmpSocket {
        pub fn new() -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Native ICMP ping requires Administrator privileges on Windows. \
                 Use the system ping command as a fallback.",
            ))
        }

        pub fn set_read_timeout(&self, _timeout: Duration) -> io::Result<()> {
            Ok(())
        }

        pub fn send_icmp(&self, _dest: &SocketAddrV4, _packet: &[u8]) -> io::Result<usize> {
            unreachable!()
        }

        pub fn recv_icmp(&self, _buf: &mut [u8]) -> io::Result<usize> {
            unreachable!()
        }
    }
}

// ── Shared ping logic ──────────────────────────────────────────────────

/// Send ICMP Echo Requests to `target` and collect results.
///
/// Returns a permission error on Windows unless running as Administrator.
pub async fn ping(
    target: IpAddr,
    count: u32,
    timeout_ms: u64,
) -> Result<Vec<PingResult>, String> {
    let target_v4 = match target {
        IpAddr::V4(v4) => v4,
        IpAddr::V6(_) => return Err("IPv6 ICMP ping is not supported yet".to_string()),
    };

    let timeout = Duration::from_millis(timeout_ms);

    tokio::task::spawn_blocking(move || native_icmp_ping(target_v4, count, timeout))
        .await
        .map_err(|e| format!("ICMP ping task failed: {}", e))?
}

fn native_icmp_ping(
    target: std::net::Ipv4Addr,
    count: u32,
    timeout: Duration,
) -> Result<Vec<PingResult>, String> {
    let socket = imp::RawIcmpSocket::new().map_err(|e| {
        format!(
            "Failed to create raw ICMP socket: {} (try running as administrator/root)",
            e
        )
    })?;

    socket.set_read_timeout(timeout).map_err(|e| format!("Failed to set socket timeout: {}", e))?;

    let dest = std::net::SocketAddrV4::new(target, 0);
    let mut results = Vec::with_capacity(count as usize);
    let identifier = rand::random::<u16>();
    let mut seq: u16 = 0;

    for _ in 0..count {
        seq = seq.wrapping_add(1);
        let packet = build_echo_request(identifier, seq);
        let send_time = Instant::now();

        if let Err(e) = socket.send_icmp(&dest, &packet) {
            results.push(PingResult {
                seq: seq as u32,
                latency_ms: -1.0,
                ttl: 0,
                status: format!("send_error: {}", e),
            });
            continue;
        }

        let mut buf = [0u8; 512];
        match socket.recv_icmp(&mut buf) {
            Ok(len) => {
                let elapsed = send_time.elapsed();
                let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

                if len < 20 {
                    results.push(PingResult {
                        seq: seq as u32,
                        latency_ms: -1.0,
                        ttl: 0,
                        status: "short_reply".to_string(),
                    });
                    continue;
                }

                let icmp_type = buf[20];
                let recv_id = u16::from_be_bytes([buf[24], buf[25]]);
                let recv_seq = u16::from_be_bytes([buf[26], buf[27]]);

                if icmp_type == 0 && recv_id == identifier && recv_seq == seq {
                    let ttl = buf[8] as u32;
                    results.push(PingResult {
                        seq: seq as u32,
                        latency_ms: elapsed_ms,
                        ttl,
                        status: "success".to_string(),
                    });
                } else if icmp_type == 3 {
                    results.push(PingResult {
                        seq: seq as u32,
                        latency_ms: -1.0,
                        ttl: 0,
                        status: "unreachable".to_string(),
                    });
                } else if icmp_type == 11 {
                    results.push(PingResult {
                        seq: seq as u32,
                        latency_ms: -1.0,
                        ttl: 0,
                        status: "timeout".to_string(),
                    });
                } else {
                    results.push(PingResult {
                        seq: seq as u32,
                        latency_ms: -1.0,
                        ttl: 0,
                        status: format!("unexpected_type_{}", icmp_type),
                    });
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                results.push(PingResult {
                    seq: seq as u32,
                    latency_ms: -1.0,
                    ttl: 0,
                    status: "timeout".to_string(),
                });
            }
            Err(e) => {
                results.push(PingResult {
                    seq: seq as u32,
                    latency_ms: -1.0,
                    ttl: 0,
                    status: format!("recv_error: {}", e),
                });
            }
        }
    }

    Ok(results)
}

/// Build an ICMP Echo Request packet with RFC 1071 checksum.
fn build_echo_request(identifier: u16, sequence: u16) -> Vec<u8> {
    let mut packet = vec![0u8; 8 + 32];

    packet[0] = 8; // type = Echo Request
    packet[1] = 0; // code = 0

    packet[4] = (identifier >> 8) as u8;
    packet[5] = (identifier & 0xff) as u8;
    packet[6] = (sequence >> 8) as u8;
    packet[7] = (sequence & 0xff) as u8;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    packet[8..16].copy_from_slice(&now.to_be_bytes());

    let checksum = compute_checksum(&packet);
    packet[2] = (checksum >> 8) as u8;
    packet[3] = (checksum & 0xff) as u8;

    packet
}

/// RFC 1071 checksum.
fn compute_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;

    while i + 1 < data.len() {
        sum = sum.wrapping_add(u16::from_be_bytes([data[i], data[i + 1]]) as u32);
        i += 2;
    }

    if i < data.len() {
        sum = sum.wrapping_add((data[i] as u32) << 8);
    }

    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    !(sum as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_all_zeros() {
        assert_eq!(compute_checksum(&[0u8; 8]), 0xffff);
    }

    #[test]
    fn test_checksum_known() {
        assert_eq!(compute_checksum(&[0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01]), 0xfffb);
    }

    #[test]
    fn test_checksum_odd_length() {
        assert_eq!(compute_checksum(&[0x00, 0x01, 0x02]), 0xfdfe);
    }

    #[test]
    fn test_build_echo_request() {
        let pkt = build_echo_request(0x1234, 1);
        assert_eq!(pkt.len(), 40);
        assert_eq!(pkt[0], 8);
        assert_eq!(pkt[1], 0);
    }

    #[test]
    fn test_build_echo_request_fields() {
        let pkt = build_echo_request(0xabcd, 5);
        assert_eq!(pkt[4], 0xab);
        assert_eq!(pkt[5], 0xcd);
        assert_eq!(pkt[6], 0x00);
        assert_eq!(pkt[7], 0x05);
    }

    #[test]
    fn test_build_echo_request_checksum_valid() {
        let pkt = build_echo_request(0x1234, 1);
        let mut check = pkt.clone();
        check[2] = 0;
        check[3] = 0;
        assert_eq!(
            u16::from_be_bytes([pkt[2], pkt[3]]),
            compute_checksum(&check),
        );
    }

    #[test]
    fn test_checksum_nonzero() {
        let pkt = build_echo_request(0x1234, 1);
        let cksum = u16::from_be_bytes([pkt[2], pkt[3]]);
        assert_ne!(cksum, 0);
        assert_ne!(cksum, 0xffff);
    }

    #[test]
    fn test_windows_stub_returns_error() {
        #[cfg(windows)]
        {
            let result = imp::RawIcmpSocket::new();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        }
    }
}
