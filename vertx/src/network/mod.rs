mod dhcp;
mod http;

use core::net::Ipv4Addr;

use embassy_executor::{Spawner, task};
use static_cell::ConstStaticCell;

use crate::hal::prelude::*;

const STATIC_ADDRESS: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 1);
const WORKERS: usize = http::WORKERS + 2; // 1 for DHCP + 1 overhead

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Station,
    AccessPoint,
}

pub(crate) type Ssid = heapless::String<32>;
pub(crate) type Password = heapless::String<64>;

#[derive(Debug)]
pub(crate) struct Credentials {
    pub(crate) ssid: Ssid,
    pub(crate) password: Password,
}

pub async fn init(
    spawner: Spawner,
    config: crate::Config,
    api: &'static crate::api::Api,
    mut hal: crate::hal::Network,
) {
    loog::info!("Starting network");

    static RESOURCES: ConstStaticCell<embassy_net::StackResources<{ WORKERS }>> =
        ConstStaticCell::new(embassy_net::StackResources::new());
    let resources = RESOURCES.take();

    let hostname = "vertx".try_into().unwrap();

    let (sta, ap) = config.network().lock(|config| {
        let sta = {
            let home = config.home();
            let ssid = home.ssid();
            let password = home.password();

            (!ssid.is_empty() && !password.is_empty()).then(|| Credentials {
                ssid: ssid.clone(),
                password: password.clone(),
            })
        };

        let ap = Credentials {
            ssid: "VerTX".try_into().unwrap(),
            password: config.password().clone(),
        };

        (sta, ap)
    });

    let seed = hal.seed();
    let (kind, driver) = hal.start(sta, ap).await;

    let config = match kind {
        Kind::Station => {
            let mut config = embassy_net::DhcpConfig::default();
            config.hostname = Some(hostname);
            embassy_net::Config::dhcpv4(config)
        }
        Kind::AccessPoint => embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
            address: embassy_net::Ipv4Cidr::new(STATIC_ADDRESS, 24),
            gateway: Some(STATIC_ADDRESS),
            dns_servers: heapless::Vec::new(),
        }),
    };

    let (stack, runner) = embassy_net::new(driver, config, resources, seed);
    spawner.must_spawn(network(runner));

    if kind == Kind::AccessPoint {
        spawner.must_spawn(dhcp::run(stack, STATIC_ADDRESS));
    }

    http::spawn_all(spawner, stack, api);

    stack.wait_link_up().await;
}

#[task]
async fn network(mut runner: embassy_net::Runner<'static, crate::hal::NetworkDriver>) -> ! {
    runner.run().await
}
