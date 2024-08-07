use core::future::Future;

use embassy_executor::{task, Spawner};
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{Ipv4Address, Ipv4Cidr, Stack, StackResources};
use embassy_time::{Duration, Timer};
use heapless::String;
use static_cell::make_static;

use crate::hal::traits::{GetWifi as _, Rng as _};

pub type Config = vertx_config::BootSnapshot<raw_config::RawConfig, crate::mutex::SingleCore>;
pub type Ssid = String<32>;
pub type Password = String<64>;

mod raw_config {
    use core::fmt;

    use heapless::String;

    #[derive(Clone, vertx_config::UpdateMut, vertx_config::Storage)]
    pub struct RawConfig {
        pub(super) hostname: String<32>,
        pub(super) password: super::Password,
        pub(super) home_ssid: super::Ssid,
        pub(super) home_password: super::Password,
    }

    impl fmt::Debug for RawConfig {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("RawConfig")
                .field("hostname", &self.hostname)
                .field("password", &"***")
                .field("home_ssid", &"***")
                .field("home_password", &"***")
                .finish()
        }
    }

    impl Default for RawConfig {
        fn default() -> Self {
            Self {
                hostname: "vertx".try_into().unwrap(),
                password: "vertxvertx".try_into().unwrap(),
                home_ssid: String::new(),
                home_password: String::new(),
            }
        }
    }

    impl RawConfig {
        pub(super) fn valid_home(&self) -> bool {
            !self.home_ssid.is_empty() && !self.home_password.is_empty()
        }
    }
}

pub enum Error {
    InvalidHomeConfig,
}

const STATIC_ADDRESS: Ipv4Address = Ipv4Address([10, 0, 0, 1]);
const STATIC_CIDR: Ipv4Cidr = Ipv4Cidr::new(STATIC_ADDRESS, 24);

pub type StackReady = impl Future<Output = ()>;

pub fn run(
    spawner: Spawner,
    is_home: bool,
    config: &'static crate::Config,
    rng: &mut crate::hal::Rng,
    get_wifi: crate::hal::GetWifi,
) -> Result<(&'static Stack<crate::hal::Wifi>, StackReady), Error> {
    let net_config = config.wifi.boot();

    let (driver, stack_config) = if is_home {
        if !net_config.valid_home() {
            log::error!("Invalid home network config");
            return Err(Error::InvalidHomeConfig);
        }

        let driver = get_wifi.home(&net_config.home_ssid, &net_config.home_password);

        let mut dhcp_config = embassy_net::DhcpConfig::default();
        dhcp_config.hostname = Some(net_config.hostname.clone());

        (driver, embassy_net::Config::dhcpv4(dhcp_config))
    } else {
        let driver = get_wifi.field("VerTX".try_into().unwrap(), &net_config.password);

        let static_config = embassy_net::StaticConfigV4 {
            address: STATIC_CIDR,
            gateway: Some(STATIC_ADDRESS),
            dns_servers: heapless::Vec::new(),
        };

        (driver, embassy_net::Config::ipv4_static(static_config))
    };

    const SOCKETS: usize = crate::configurator::TASKS + 2; // dhcp + spare
    let resources = make_static!(StackResources::<{ SOCKETS }>::new());
    let stack: &'static _ = make_static!(Stack::new(driver, stack_config, resources, rng.u64()));

    if !is_home {
        spawner.must_spawn(dhcp_server(stack));
    }

    spawner.must_spawn(network(stack));

    let stack_ready = async move {
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
    };

    Ok((stack, stack_ready))
}

#[task]
async fn network(stack: &'static Stack<crate::hal::Wifi>) -> ! {
    stack.run().await
}

#[task]
async fn dhcp_server(stack: &'static Stack<crate::hal::Wifi>) {
    use edge_dhcp::Ipv4Addr;

    const fn convert_address(embassy: embassy_net::Ipv4Address) -> edge_dhcp::Ipv4Addr {
        let octets = embassy.0;
        Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3])
    }

    const LEASES: usize = 2;
    const ADDRESS: Ipv4Addr = convert_address(STATIC_ADDRESS);

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

    let mut server = edge_dhcp::server::Server::<{ LEASES }>::new(ADDRESS);
    let server_options = edge_dhcp::server::ServerOptions {
        ip: ADDRESS,
        gateways: &[ADDRESS],
        subnet: Some(convert_address(STATIC_CIDR.netmask())),
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
