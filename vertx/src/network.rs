use embassy_executor::Spawner;

pub type Config = vertx_config::BootSnapshot<raw_config::RawConfig, crate::mutex::SingleCore>;

mod raw_config {
    use core::fmt;

    use heapless::String;
    use vertx_network::{Password, Ssid};

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

const STATIC_ADDRESS: [u8; 4] = [10, 0, 0, 1];

pub async fn run(
    spawner: Spawner,
    is_home: bool,
    config: &'static crate::Config,
    api: &'static crate::api::Api,
    #[cfg(feature = "network-native")] rng: &mut crate::hal::Rng,
    #[cfg(feature = "network-native")] network: crate::hal::Network,
    #[cfg(feature = "network-backpack")] backpack: crate::backpack::Backpack,
) -> Result<(), Error> {
    let config = config.network.boot();

    let server_config = if is_home {
        if !config.valid_home() {
            loog::error!("Invalid home network config");
            return Err(Error::InvalidHomeConfig);
        }

        vertx_network::Config::Home {
            ssid: config.home_ssid.clone(),
            password: config.home_password.clone(),
            hostname: "vertx".try_into().unwrap(),
        }
    } else {
        vertx_network::Config::Field {
            ssid: "VerTX".try_into().unwrap(),
            password: config.password.clone(),
            address: STATIC_ADDRESS,
        }
    };

    #[cfg(feature = "network-native")]
    native::run(
        spawner,
        server_config,
        rand::RngCore::next_u64(rng),
        network,
        api,
    )
    .await;
    #[cfg(not(feature = "network-native"))]
    let _ = spawner;

    #[cfg(feature = "network-backpack")]
    backpack.start_network(server_config, api).await;

    Ok(())
}

#[cfg(feature = "network-native")]
mod native {
    use embassy_executor::{task, Spawner};
    use static_cell::make_static;
    use vertx_server::tasks::DhcpContext;

    use crate::api::Api;

    const WORKERS: usize = 8;

    vertx_server::get_router!(get_router -> Router<Api>);
    type Context = vertx_server::Context<crate::hal::NetworkDriver>;

    pub(super) async fn run(
        spawner: Spawner,
        config: vertx_network::Config,
        seed: u64,
        hal: crate::hal::Network,
        api: &'static Api,
    ) {
        let resources = make_static!(vertx_server::Resources::<{ WORKERS + 2 }>::new());
        let (context, dhcp_context, wait) = vertx_server::init(resources, config, seed, hal);
        let context = make_static!(context);

        spawner.must_spawn(network(context));

        if let Some(dhcp_context) = dhcp_context {
            spawner.must_spawn(dhcp(context, dhcp_context));
        }

        let router = make_static!(get_router());
        for id in 0..WORKERS {
            spawner.must_spawn(http(id, context, router, api));
        }

        wait.wait_for_network(context).await;
    }

    #[task]
    async fn network(context: &'static Context) -> ! {
        vertx_server::tasks::network(context).await
    }

    #[task]
    async fn dhcp(context: &'static Context, dhcp: DhcpContext) -> ! {
        vertx_server::tasks::dhcp(context, dhcp).await
    }

    #[task(pool_size = WORKERS)]
    async fn http(
        id: usize,
        context: &'static Context,
        router: &'static Router,
        api: &'static Api,
    ) -> ! {
        vertx_server::tasks::http(id, context, router, api).await
    }
}
