#[cfg(not(any(feature = "network-usb-ethernet", feature = "network-wifi")))]
compile_error!("At least one network implementation must be enabled");

mod dhcp;
mod driver;
mod http;
#[cfg(feature = "network-wifi")]
pub(crate) mod wifi;

use core::net::Ipv4Addr;

use embassy_executor::{Spawner, task};
use static_cell::StaticCell;

const STATIC_ADDRESS: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 1);
const WORKERS: usize = http::WORKERS + 2; // 1 for DHCP + 1 overhead

pub enum Init {
    #[cfg(feature = "network-usb-ethernet")]
    Ethernet(crate::usb::ncm_cdc::NetDriver),
    #[cfg(feature = "network-wifi")]
    Wifi(crate::hal::Wifi),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(unused)]
pub enum Kind {
    StaticIp,
    DhcpClient,
}

pub async fn init(
    spawner: Spawner,
    config: crate::Config,
    api: &'static crate::configurator::Api,
    get_seed: crate::hal::GetNetworkSeed,
    init: Init,
) {
    loog::info!("Starting network");

    static RESOURCES: StaticCell<embassy_net::StackResources<{ WORKERS }>> = StaticCell::new();
    let resources = RESOURCES.init_with(embassy_net::StackResources::new);

    let (driver, kind) = match init {
        #[cfg(feature = "network-usb-ethernet")]
        Init::Ethernet(driver) => (driver::Driver::Ethernet(driver), Kind::StaticIp),
        #[cfg(feature = "network-wifi")]
        Init::Wifi(wifi) => {
            let (driver, kind) = wifi::init(config, wifi).await;
            (driver::Driver::Wifi(driver), kind.into())
        }
    };

    let config = match kind {
        Kind::StaticIp => embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
            address: embassy_net::Ipv4Cidr::new(STATIC_ADDRESS, 24),
            gateway: Some(STATIC_ADDRESS),
            dns_servers: heapless::Vec::new(),
        }),
        Kind::DhcpClient => {
            let hostname = config.network().hostname().lock(Clone::clone);
            let mut config = embassy_net::DhcpConfig::default();
            config.hostname = Some(hostname);
            embassy_net::Config::dhcpv4(config)
        }
    };

    let (stack, runner) = embassy_net::new(driver, config, resources, get_seed());
    spawner.must_spawn(network(runner));

    if kind == Kind::StaticIp {
        spawner.must_spawn(dhcp::run(stack, STATIC_ADDRESS));
    }

    http::spawn_all(spawner, stack, api);

    stack.wait_link_up().await;
}

#[task]
async fn network(mut runner: embassy_net::Runner<'static, driver::Driver>) -> ! {
    runner.run().await
}
