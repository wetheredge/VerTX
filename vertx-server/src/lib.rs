#![no_std]
#![allow(private_interfaces, missing_debug_implementations)]

extern crate alloc;

mod configurator;

use embassy_net::driver::Driver;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{Ipv4Address, Ipv4Cidr};
use embassy_time::{Duration, Timer};
use vertx_network::Config;

pub use self::configurator::Router;

#[doc(hidden)]
pub mod private {
    pub use picoserve;

    pub use crate::configurator::api::UpgradeHandler as ApiUpgradeHandler;
    pub use crate::configurator::AssetsRouter;
}

#[macro_export]
macro_rules! get_router {
    ($vis:vis $f:ident -> $t:ident<$api:ty>) => {
        $vis type $t = $crate::private::picoserve::Router<
            impl $crate::private::picoserve::routing::PathRouter<$api>,
            $api,
        >;

        $vis fn $f() -> $t {
            $crate::private::picoserve::Router::new()
                .nest_service("", $crate::private::AssetsRouter)
                .nest_service("/api", $crate::private::ApiUpgradeHandler)
        }
    };
}

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

#[repr(transparent)]
pub struct Context<D: Driver>(embassy_net::Stack<D>);

pub async fn init<H: vertx_network::Hal, const SOCKETS: usize>(
    resources: &'static mut Resources<SOCKETS>,
    config: Config,
    seed: u64,
    hal: H,
) -> (Context<H::Driver>, Option<tasks::DhcpContext>) {
    let is_home = matches!(config, Config::Home { .. });

    let (driver, stack_config, dhcp_address) = match config {
        Config::Home {
            ssid,
            password,
            hostname,
        } => {
            let driver = hal.home(ssid, password);

            let mut dhcp_config = embassy_net::DhcpConfig::default();
            dhcp_config.hostname = Some(hostname);

            (driver, embassy_net::Config::dhcpv4(dhcp_config), None)
        }
        Config::Field {
            ssid,
            password,
            address: raw_address,
        } => {
            let driver = hal.field(ssid, password);

            let address = Ipv4Address(raw_address);
            let static_config = embassy_net::StaticConfigV4 {
                address: Ipv4Cidr::new(address, 24),
                gateway: Some(address),
                dns_servers: heapless::Vec::new(),
            };

            let config = embassy_net::Config::ipv4_static(static_config);

            (driver, config, Some(raw_address))
        }
    };

    let Resources(resources) = resources;
    let stack = embassy_net::Stack::new(driver, stack_config, resources, seed);

    if is_home {
        stack.wait_config_up().await;
    } else {
        loop {
            if stack.is_link_up() {
                break;
            }
            Timer::after(Duration::from_millis(500)).await;
        }
    }

    let context = Context(stack);

    (context, dhcp_address.map(tasks::DhcpContext))
}

pub mod tasks {
    use super::*;
    pub use crate::configurator::worker as http;

    pub async fn network<D: Driver>(context: &Context<D>) -> ! {
        context.0.run().await
    }

    pub struct DhcpContext(pub(crate) [u8; 4]);

    pub async fn dhcp<D: Driver>(context: &Context<D>, dhcp_context: DhcpContext) -> ! {
        use edge_dhcp::Ipv4Addr;

        const LEASES: usize = 2;

        let Context(stack) = context;
        let DhcpContext(address) = dhcp_context;

        let address = Ipv4Addr::new(address[0], address[1], address[2], address[3]);
        let mask = Ipv4Addr::new(255, 255, 255, 0);

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

        let mut server = edge_dhcp::server::Server::<{ LEASES }>::new(address);
        let server_options = edge_dhcp::server::ServerOptions {
            ip: address,
            gateways: &[address],
            subnet: Some(mask),
            dns: &[],
            lease_duration_secs: 60 * 5,
        };

        let mut buffer = [0; 1536];
        loop {
            let (len, remote) = socket.recv_from(&mut buffer).await.unwrap();
            let packet = &buffer[..len];

            let request = match edge_dhcp::Packet::decode(packet) {
                Ok(decoded) => decoded,
                Err(err) => {
                    log::warn!("Failed to decode DHCP packet: {err:?}");
                    continue;
                }
            };

            let mut options = edge_dhcp::Options::buf();
            if let Some(reply) = server.handle_request(&mut options, &server_options, &request) {
                let remote = if request.broadcast || remote.addr.is_unspecified() {
                    embassy_net::IpEndpoint::new(Ipv4Address::BROADCAST.into_address(), remote.port)
                } else {
                    remote
                };

                let reply = reply.encode(&mut buffer).unwrap();
                socket.send_to(reply, remote).await.unwrap();
            };
        }
    }
}
