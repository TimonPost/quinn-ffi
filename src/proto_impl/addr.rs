use std::{
    net,
    net::{
        Ipv4Addr,
        SocketAddr,
        SocketAddrV4,
    },
};

/// IpAddress that is FFI safe.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct IpAddr {
    /// The port
    port: u16,
    // The address bytes
    // Right now only ip4 is supported
    address: [u8; 4],
}

impl From<SocketAddr> for IpAddr {
    /// From `SocketAddr` to FFI-safe `IpAddr`
    fn from(addr: SocketAddr) -> Self {
        let address_bytes = match addr.ip() {
            net::IpAddr::V4(ip) => ip.octets(),
            net::IpAddr::V6(_ip) => panic!("not supported"),
        };

        IpAddr {
            port: addr.port(),
            address: address_bytes,
        }
    }
}

impl Into<SocketAddr> for IpAddr {
    /// From FFI-safe `IpAddr` to `SocketAddr`
    fn into(self) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(
                self.address[0],
                self.address[1],
                self.address[2],
                self.address[3],
            ),
            self.port,
        ))
    }
}
