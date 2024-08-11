use embassy_executor::{task, Spawner};
use static_cell::make_static;
use vertx_server::tasks::DhcpContext;

use crate::api::Api;
use crate::hal::traits::Rng as _;

pub type Config = vertx_config::BootSnapshot<raw_config::RawConfig, crate::mutex::SingleCore>;

mod raw_config {
    use core::fmt;

    use heapless::String;
    use vertx_server::{Password, Ssid};

    #[derive(Clone, vertx_config::UpdateMut, vertx_config::Storage)]
    pub struct RawConfig {
        pub(super) hostname: String<32>,
        pub(super) password: Password,
        pub(super) home_ssid: Ssid,
        pub(super) home_password: Password,
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

const WORKERS: usize = 8;
const STATIC_ADDRESS: [u8; 4] = [10, 0, 0, 1];

pub fn run(
    spawner: Spawner,
    is_home: bool,
    config: &'static crate::Config,
    rng: &mut crate::hal::Rng,
    network: crate::hal::Network,
    api: &'static Api,
) -> Result<(), Error> {
    let config = config.network.boot();

    let server_config = if is_home {
        if !config.valid_home() {
            log::error!("Invalid home network config");
            return Err(Error::InvalidHomeConfig);
        }

        vertx_server::Config::Home {
            ssid: config.home_ssid.clone(),
            password: config.home_password.clone(),
            hostname: "vertx".try_into().unwrap(),
        }
    } else {
        vertx_server::Config::Field {
            ssid: "VerTX".try_into().unwrap(),
            password: config.password.clone(),
            address: STATIC_ADDRESS,
        }
    };

    spawner.must_spawn(start_tasks(spawner, server_config, rng.u64(), network, api));
    Ok(())
}

vertx_server::get_router!(get_router -> Router<Api>);
type Context = vertx_server::Context<crate::hal::NetworkDriver>;

#[task]
async fn start_tasks(
    spawner: Spawner,
    config: vertx_server::Config,
    seed: u64,
    hal: crate::hal::Network,
    api: &'static Api,
) {
    let resources = make_static!(vertx_server::Resources::<{ WORKERS + 2 }>::new());
    let (context, dhcp_context) = vertx_server::init(resources, config, seed, hal).await;
    let context = make_static!(context);

    spawner.must_spawn(network(context));

    if let Some(dhcp_context) = dhcp_context {
        spawner.must_spawn(dhcp(context, dhcp_context));
    }

    let router = make_static!(get_router());
    for id in 0..WORKERS {
        spawner.must_spawn(http(id, context, router, api));
    }
}

#[task]
async fn network(context: &'static Context) -> ! {
    vertx_server::tasks::network(context).await
}

#[task]
async fn dhcp(context: &'static Context, dhcp: DhcpContext) -> ! {
    vertx_server::tasks::dhcp(context, dhcp).await
}

#[task]
async fn http(
    id: usize,
    context: &'static Context,
    router: &'static Router,
    api: &'static Api,
) -> ! {
    vertx_server::tasks::http(id, context, router, api).await
}
