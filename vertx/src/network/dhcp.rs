use core::net::Ipv4Addr;

use edge_dhcp::server;
use embassy_executor::task;
use embassy_net::Stack;
use embassy_net::udp::{PacketMetadata, UdpSocket};

#[task]
pub(super) async fn run(stack: Stack<'static>, address: Ipv4Addr) -> ! {
    const LEASES: usize = 2;

    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx_buffer = [0; 1536];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_buffer = [0; 1536];

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    socket.bind(67).unwrap();

    let mut server = server::Server::<_, { LEASES }>::new_with_et(address);
    let gateways = &[address];
    let server_options = {
        let mut options = server::ServerOptions::new(address, None);
        options.gateways = gateways;
        // TODO: set captive_url?
        options
    };

    let mut buffer = [0; 1536];
    loop {
        let (len, meta) = socket.recv_from(&mut buffer).await.unwrap();
        let packet = &buffer[..len];
        let remote = meta.endpoint;

        let request = match edge_dhcp::Packet::decode(packet) {
            Ok(decoded) => decoded,
            Err(err) => {
                loog::warn!(
                    "Failed to decode DHCP packet: {:?}",
                    loog::Debug2Format(&err),
                );
                continue;
            }
        };

        let mut options = edge_dhcp::Options::buf();
        if let Some(reply) = server.handle_request(&mut options, &server_options, &request) {
            let remote = if request.broadcast || remote.addr.is_unspecified() {
                embassy_net::IpEndpoint::new(
                    embassy_net::Ipv4Address::BROADCAST.into(),
                    remote.port,
                )
            } else {
                remote
            };

            let reply = reply.encode(&mut buffer).unwrap();
            socket.send_to(reply, remote).await.unwrap();
        }
    }
}
