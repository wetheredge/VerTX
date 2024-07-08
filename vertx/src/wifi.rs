use embassy_executor::{task, Spawner};
use embassy_net::{Stack, StackResources};
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
        pub(super) password: String<64>,
        pub(super) home_ssid: String<32>,
        pub(super) home_password: String<64>,
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
                home_ssid: env!("WIFI_SSID").try_into().unwrap(),
                home_password: env!("WIFI_PASSWORD").try_into().unwrap(),
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

pub fn run(
    spawner: Spawner,
    is_home: bool,
    config: &'static crate::Config,
    rng: &mut crate::hal::Rng,
    get_wifi: crate::hal::GetWifi,
) -> Result<&'static Stack<crate::hal::Wifi>, Error> {
    let net_config = config.wifi.boot();

    let driver = if is_home {
        if !net_config.valid_home() {
            log::error!("Invalid home network config");
            return Err(Error::InvalidHomeConfig);
        }

        get_wifi.home(&net_config.home_ssid, &net_config.home_password)
    } else {
        get_wifi.field("VerTX".try_into().unwrap(), &net_config.password)
    };

    let mut dhcp_config = embassy_net::DhcpConfig::default();
    dhcp_config.hostname = Some(net_config.hostname.clone());

    let stack_config = embassy_net::Config::dhcpv4(dhcp_config);
    let resources = make_static!(StackResources::<{ crate::configurator::TASKS + 1 }>::new());
    let stack = make_static!(Stack::new(driver, stack_config, resources, rng.u64()));

    spawner.must_spawn(network(stack));

    Ok(stack)
}

#[task]
async fn network(stack: &'static Stack<crate::hal::Wifi>) -> ! {
    stack.run().await
}
