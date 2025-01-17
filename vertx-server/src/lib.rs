#![no_std]
#![expect(missing_debug_implementations)]

extern crate alloc;

mod http;

use embassy_net::Ipv4Cidr;
pub use embassy_net::{Runner, Stack};
use vertx_network::{Config, NetworkKind};

/// Memory resources for a network stack
///
/// `SOCKETS` should be 2 greater than the number of http worker tasks. (1 for
/// DHCP and 1 spare)
#[repr(transparent)]
pub struct Resources<const SOCKETS: usize>(embassy_net::StackResources<SOCKETS>);

impl<const WORKERS: usize> Resources<WORKERS> {
    pub const fn new() -> Self {
        Self(embassy_net::StackResources::new())
    }
}

impl<const SOCKETS: usize> Default for Resources<SOCKETS> {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn init<H: vertx_network::Hal, const SOCKETS: usize>(
    resources: &'static mut Resources<SOCKETS>,
    config: Config,
    seed: u64,
    hal: H,
) -> (
    Stack<'static>,
    embassy_net::Runner<'static, H::Driver>,
    Option<tasks::DhcpContext>,
) {
    let (home, hostname) = if let Some(home) = config.home {
        (Some(home.credentials), home.hostname)
    } else {
        (None, heapless::String::new())
    };

    let (network, driver) = hal.start(home, config.field.credentials).await;

    let (stack_config, dhcp_address) = match network {
        NetworkKind::Home => {
            let mut dhcp_config = embassy_net::DhcpConfig::default();
            dhcp_config.hostname = Some(hostname);

            (embassy_net::Config::dhcpv4(dhcp_config), None)
        }
        NetworkKind::Field => {
            let address = config.field.address;

            let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
                address: Ipv4Cidr::new(address, 24),
                gateway: Some(address),
                dns_servers: heapless::Vec::new(),
            });

            (config, Some(address))
        }
    };

    let Resources(resources) = resources;
    let (stack, runner) = embassy_net::new(driver, stack_config, resources, seed);

    (stack, runner, dhcp_address.map(tasks::DhcpContext))
}

pub mod tasks {
    use core::net::Ipv4Addr;

    use embassy_net::tcp::TcpSocket;
    use embassy_net::udp::{PacketMetadata, UdpSocket};
    use embassy_net::Stack;
    use vertx_network::Api;

    pub async fn http<A: Api>(id: usize, stack: Stack<'_>, api: &A) -> ! {
        const TCP_BUFFER: usize = 1024;
        const HTTP_BUFFER: usize = 2048;

        let mut rx_buffer = [0; TCP_BUFFER];
        let mut tx_buffer = [0; TCP_BUFFER];
        let mut http_buffer = [0; HTTP_BUFFER];

        loop {
            let mut tcp = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

            if let Err(err) = tcp.accept(80).await {
                loog::warn!("server({id}): Accept error: {err:?}");
                continue;
            }

            let (rx, tx) = tcp.split();
            if let Err(err) = crate::http::run(rx, tx, &mut http_buffer, api).await {
                loog::error!("server({id}): Error: {err:?}");
            }
        }
    }

    pub struct DhcpContext(pub(crate) Ipv4Addr);

    pub async fn dhcp(stack: Stack<'_>, dhcp_context: DhcpContext) -> ! {
        const LEASES: usize = 2;

        let DhcpContext(address) = dhcp_context;

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

        let mut server = edge_dhcp::server::Server::<_, { LEASES }>::new_with_et(address);
        let gateways = &[address];
        let server_options = {
            let mut options = edge_dhcp::server::ServerOptions::new(address, None);
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
                        loog::Debug2Format(&err)
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
            };
        }
    }
}
