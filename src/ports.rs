//! TCP listener table probe, backing external-server detection.
//!
//! A server config may declare the TCP `port` it listens on. If that port is
//! LISTENing but the process wasn't spawned by us, it was started externally
//! (a terminal, an AI dev session, ...) and the tray shows it as [external].

use std::collections::HashMap;

/// Snapshot of all listening TCP ports (IPv4 + IPv6) mapped to the owning PID.
/// If a port has both an IPv4 and an IPv6 listener, the first insertion wins —
/// in practice both belong to the same process, so either PID works for taskkill.
#[cfg(windows)]
pub fn listening_ports() -> HashMap<u16, u32> {
    let mut map = HashMap::new();
    win::collect_v4(&mut map);
    win::collect_v6(&mut map);
    map
}

/// Non-Windows stub: no detection, everything reads as not listening.
#[cfg(not(windows))]
pub fn listening_ports() -> HashMap<u16, u32> {
    HashMap::new()
}

/// Whether anything is currently listening on `port`. Used to wait for a port
/// to be released after killing an external process (no Child handle to wait on).
pub fn port_listening(port: u16) -> bool {
    listening_ports().contains_key(&port)
}

#[cfg(windows)]
mod win {
    use std::collections::HashMap;

    use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR, TRUE};
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID, MIB_TCPROW_OWNER_PID,
        MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    };
    use windows::Win32::Networking::WinSock::{AF_INET, AF_INET6};

    /// Fetch a TCP owner-PID listener table for one address family via the
    /// two-call sizing pattern. Returns the raw table buffer, or None on failure.
    /// Retries a few times because the table can grow between the sizing call
    /// and the fetch.
    fn fetch_table(family: u32) -> Option<Vec<u8>> {
        unsafe {
            let mut size: u32 = 0;
            let rc = GetExtendedTcpTable(
                None,
                &mut size,
                TRUE,
                family,
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            );
            if rc != NO_ERROR.0 && rc != ERROR_INSUFFICIENT_BUFFER.0 {
                return None;
            }

            for _ in 0..3 {
                let mut buf = vec![0u8; size.max(16) as usize];
                let rc = GetExtendedTcpTable(
                    Some(buf.as_mut_ptr().cast()),
                    &mut size,
                    TRUE,
                    family,
                    TCP_TABLE_OWNER_PID_LISTENER,
                    0,
                );
                if rc == ERROR_INSUFFICIENT_BUFFER.0 {
                    continue; // size was updated; retry with a larger buffer
                }
                if rc != NO_ERROR.0 {
                    return None;
                }
                return Some(buf);
            }
            None
        }
    }

    pub(super) fn collect_v4(out: &mut HashMap<u16, u32>) {
        let Some(buf) = fetch_table(AF_INET.0 as u32) else {
            return;
        };
        unsafe {
            let table = buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID;
            let rows = std::ptr::addr_of!((*table).table) as *const MIB_TCPROW_OWNER_PID;
            let offset = rows as usize - table as usize;
            // Bound by the actual buffer size in case the reported count overruns it
            let max_rows =
                buf.len().saturating_sub(offset) / std::mem::size_of::<MIB_TCPROW_OWNER_PID>();
            let n = ((*table).dwNumEntries as usize).min(max_rows);
            for i in 0..n {
                let row = &*rows.add(i);
                // dwLocalPort is network byte order in the low 16 bits
                let port = u16::from_be(row.dwLocalPort as u16);
                out.entry(port).or_insert(row.dwOwningPid);
            }
        }
    }

    pub(super) fn collect_v6(out: &mut HashMap<u16, u32>) {
        let Some(buf) = fetch_table(AF_INET6.0 as u32) else {
            return;
        };
        unsafe {
            let table = buf.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID;
            let rows = std::ptr::addr_of!((*table).table) as *const MIB_TCP6ROW_OWNER_PID;
            let offset = rows as usize - table as usize;
            let max_rows =
                buf.len().saturating_sub(offset) / std::mem::size_of::<MIB_TCP6ROW_OWNER_PID>();
            let n = ((*table).dwNumEntries as usize).min(max_rows);
            for i in 0..n {
                let row = &*rows.add(i);
                let port = u16::from_be(row.dwLocalPort as u16);
                out.entry(port).or_insert(row.dwOwningPid);
            }
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn detects_own_listener() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let map = super::listening_ports();
        assert_eq!(map.get(&port), Some(&std::process::id()));
        assert!(super::port_listening(port));
    }

    #[test]
    fn detects_own_v6_listener() {
        let listener = std::net::TcpListener::bind("[::1]:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let map = super::listening_ports();
        assert_eq!(map.get(&port), Some(&std::process::id()));
    }
}
