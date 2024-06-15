use core::fmt;

use embassy_executor::{task, Spawner};
use embassy_net::{Stack, StackResources};
use heapless::String;
use static_cell::make_static;

use crate::hal::traits::Rng as _;

pub type Config = vertx_config::BootSnapshot<RawConfig, crate::mutex::SingleCore>;

#[derive(Clone, vertx_config::UpdateMut, vertx_config::Storage)]
pub struct RawConfig {
    hostname: String<32>,
    password: String<64>,
    home_ssid: String<32>,
    home_password: String<64>,
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
            password: String::new(), // TODO
            home_ssid: env!("WIFI_SSID").try_into().unwrap(),
            home_password: env!("WIFI_PASSWORD").try_into().unwrap(),
        }
    }
}

impl RawConfig {
    fn valid_home(&self) -> bool {
        !self.home_ssid.is_empty() && !self.home_password.is_empty()
    }
}

pub fn run(
    spawner: Spawner,
    config: &'static crate::Config,
    rng: &mut crate::hal::Rng,
    get_net_driver: crate::hal::GetNetDriver,
) -> &'static Stack<crate::hal::NetDriver> {
    let config = config.wifi.boot();
    assert!(config.valid_home());
    let driver = get_net_driver(&config.home_ssid, &config.home_password);

    let mut dhcp_config = embassy_net::DhcpConfig::default();
    dhcp_config.hostname = Some(config.hostname.clone());

    let stack_config = embassy_net::Config::dhcpv4(dhcp_config);
    let resources = make_static!(StackResources::<{ crate::configurator::TASKS + 1 }>::new());
    let stack = make_static!(Stack::new(driver, stack_config, resources, rng.u64()));

    spawner.must_spawn(network(stack));

    stack
}

#[task]
async fn network(stack: &'static Stack<crate::hal::NetDriver>) -> ! {
    stack.run().await
}
