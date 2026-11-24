//! The machine's network adapters, so a download can pick which ones carry it.
//!
//! Windows is asked for the adapter list through `GetAdaptersAddresses`; each
//! adapter's first IPv4 address is the one a socket binds to. No extra crate is
//! pulled in: the two structures we read are declared here, and the rest of the
//! record is ignored.

use std::net::Ipv4Addr;

/// One usable network: the friendly name Windows shows and its IPv4 address.
#[derive(Clone, Debug, PartialEq)]
pub struct Adapter {
    pub name: String,
    pub description: String,
    pub ip: Ipv4Addr,
}

impl Adapter {
    /// The label used in the picker and the sidebar: `Ethernet · 192.168.1.74`.
    pub fn label(&self) -> String {
        format!("{} · {}", self.name, self.ip)
    }

    /// Why streams sent out of this adapter would fail, when that is knowable
    /// without opening a socket. Virtual switches and tunnels carry an address
    /// but have no route of their own to an internet host.
    pub fn route_hint(&self) -> Option<&'static str> {
        let text = format!("{} {}", self.name, self.description).to_lowercase();
        if text.contains("hyper-v") || text.contains("vethernet") {
            Some("a Hyper-V virtual switch, which has no route to the internet")
        } else if text.contains("tap") || text.contains("vpn") || text.contains("wireguard") {
            Some("a tunnel adapter, so its streams fail whenever the tunnel is down")
        } else if text.contains("loopback") || self.ip.is_loopback() {
            Some("a loopback address, which never leaves this machine")
        } else if text.contains("bluetooth") {
            Some("a Bluetooth adapter, which this build does not route TCP over")
        } else {
            None
        }
    }
}

/// Every adapter that is up, not loopback, and has an IPv4 address.
pub fn list() -> Vec<Adapter> {
    imp::list()
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::net::Ipv4Addr;

    use super::Adapter;

    const AF_UNSPEC: u32 = 0;
    const AF_INET: u16 = 2;
    const GAA_FLAG_SKIP_ANYCAST: u32 = 0x2;
    const GAA_FLAG_SKIP_MULTICAST: u32 = 0x4;
    const GAA_FLAG_SKIP_DNS_SERVER: u32 = 0x8;
    const ERROR_BUFFER_OVERFLOW: u32 = 111;
    const IF_OPER_STATUS_UP: u32 = 1;
    const IF_TYPE_SOFTWARE_LOOPBACK: u32 = 24;

    #[repr(C)]
    struct SockAddrIn {
        family: u16,
        port: u16,
        address: [u8; 4],
        zero: [u8; 8],
    }

    #[repr(C)]
    struct SocketAddress {
        sockaddr: *mut SockAddrIn,
        length: i32,
    }

    #[repr(C)]
    struct UnicastAddress {
        length: u32,
        flags: u32,
        next: *mut UnicastAddress,
        address: SocketAddress,
    }

    #[repr(C)]
    struct AdapterAddresses {
        length: u32,
        if_index: u32,
        next: *mut AdapterAddresses,
        adapter_name: *const u8,
        first_unicast: *mut UnicastAddress,
        first_anycast: *mut c_void,
        first_multicast: *mut c_void,
        first_dns: *mut c_void,
        dns_suffix: *const u16,
        description: *const u16,
        friendly_name: *const u16,
        physical_address: [u8; 8],
        physical_address_length: u32,
        flags: u32,
        mtu: u32,
        if_type: u32,
        oper_status: u32,
    }

    #[link(name = "iphlpapi")]
    extern "system" {
        fn GetAdaptersAddresses(
            family: u32,
            flags: u32,
            reserved: *mut c_void,
            addresses: *mut AdapterAddresses,
            size: *mut u32,
        ) -> u32;
    }

    fn wide_to_string(ptr: *const u16) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let mut len = 0usize;
        // The buffers the API hands back are NUL terminated.
        unsafe {
            while *ptr.add(len) != 0 && len < 512 {
                len += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
        }
    }

    fn ipv4_of(mut unicast: *mut UnicastAddress) -> Option<Ipv4Addr> {
        while !unicast.is_null() {
            let entry = unsafe { &*unicast };
            let sockaddr = entry.address.sockaddr;
            if !sockaddr.is_null() {
                let sockaddr = unsafe { &*sockaddr };
                if sockaddr.family == AF_INET {
                    let [a, b, c, d] = sockaddr.address;
                    let ip = Ipv4Addr::new(a, b, c, d);
                    if !ip.is_loopback() && !ip.is_unspecified() {
                        return Some(ip);
                    }
                }
            }
            unicast = entry.next;
        }
        None
    }

    pub fn list() -> Vec<Adapter> {
        let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
        let mut size = 16 * 1024u32;
        let mut buffer: Vec<u64> = Vec::new();
        let mut found = ERROR_BUFFER_OVERFLOW;
        // Ask for the size first, then for the list itself.
        for _ in 0..4 {
            buffer = vec![0u64; (size as usize + 7) / 8];
            found = unsafe {
                GetAdaptersAddresses(
                    AF_UNSPEC,
                    flags,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut AdapterAddresses,
                    &mut size,
                )
            };
            if found != ERROR_BUFFER_OVERFLOW {
                break;
            }
        }
        if found != 0 {
            return Vec::new();
        }

        let mut adapters = Vec::new();
        let mut current = buffer.as_ptr() as *const AdapterAddresses;
        while !current.is_null() {
            let entry = unsafe { &*current };
            let up = entry.oper_status == IF_OPER_STATUS_UP;
            let loopback = entry.if_type == IF_TYPE_SOFTWARE_LOOPBACK;
            if up && !loopback {
                if let Some(ip) = ipv4_of(entry.first_unicast) {
                    let name = wide_to_string(entry.friendly_name);
                    let description = wide_to_string(entry.description);
                    adapters.push(Adapter {
                        name: if name.is_empty() {
                            description.clone()
                        } else {
                            name
                        },
                        description,
                        ip,
                    });
                }
            }
            current = entry.next;
        }
        adapters.sort_by(|a, b| a.name.cmp(&b.name).then(a.ip.cmp(&b.ip)));
        adapters.dedup_by(|a, b| a.ip == b.ip);
        adapters
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Adapter;

    pub fn list() -> Vec<Adapter> {
        Vec::new()
    }
}
