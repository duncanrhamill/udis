use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use socket2::{Domain, Protocol, Socket, Type};

use crate::error::Error;

/// Multicast port used for udis traffic
pub const MULTICAST_PORT: u16 = 8787;

/// Multicast address used for udis traffic, note we use IPv4 due to greater support in most
/// networks.
pub static MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 87);

/// Build the multicast socket for use in udis endpoints
pub fn build_multicast_socket() -> Result<(SocketAddr, Socket), Error> {
    // Get the addresses
    let disc_addr = SocketAddrV4::new(MULTICAST_ADDR, MULTICAST_PORT);

    // Build the multicast socket
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;
    socket.join_multicast_v4(&MULTICAST_ADDR, &Ipv4Addr::UNSPECIFIED)?;
    socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, MULTICAST_PORT).into())?;

    Ok((disc_addr.into(), socket))
}

#[cfg(test)]
mod tests {
    use crate::net::MULTICAST_ADDR;

    #[test]
    fn test_multicast() {
        assert!(MULTICAST_ADDR.is_multicast());
    }
}
