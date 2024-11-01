use embassy_executor::Spawner;

pub enum Error {
    InvalidHomeConfig,
}

const STATIC_ADDRESS: [u8; 4] = [10, 0, 0, 1];

pub async fn run(
    spawner: Spawner,
    is_home: bool,
    config: crate::Config,
    api: &'static crate::api::Api,
    #[cfg(feature = "network-native")] rng: &mut crate::hal::Rng,
    #[cfg(feature = "network-native")] network: crate::hal::Network,
    #[cfg(feature = "network-backpack")] backpack: crate::backpack::Backpack,
) -> Result<(), Error> {
    let server_config = config.network().lock(|config| {
        if is_home {
            let home = config.home();
            let ssid = home.ssid();
            let password = home.password();

            if ssid.is_empty() || password.is_empty() {
                loog::error!("Invalid home network config");
                Err(Error::InvalidHomeConfig)
            } else {
                Ok(vertx_network::Config::Home {
                    ssid: ssid.clone(),
                    password: password.clone(),
                    hostname: "vertx".try_into().unwrap(),
                })
            }
        } else {
            Ok(vertx_network::Config::Field {
                ssid: "VerTX".try_into().unwrap(),
                password: config.password().clone(),
                address: STATIC_ADDRESS,
            })
        }
    })?;

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
    use static_cell::{ConstStaticCell, StaticCell};
    use vertx_server::tasks::DhcpContext;

    use crate::api::Api;

    const WORKERS: usize = 8;

    type Context = vertx_server::Context<crate::hal::NetworkDriver>;

    pub(super) async fn run(
        spawner: Spawner,
        config: vertx_network::Config,
        seed: u64,
        hal: crate::hal::Network,
        api: &'static Api,
    ) {
        static RESOURCES: ConstStaticCell<vertx_server::Resources<{ WORKERS + 2 }>> =
            ConstStaticCell::new(vertx_server::Resources::new());
        let (context, dhcp_context, wait) = vertx_server::init(RESOURCES.take(), config, seed, hal);
        static CONTEXT: StaticCell<Context> = StaticCell::new();
        let context = CONTEXT.init(context);

        spawner.must_spawn(network(context));

        if let Some(dhcp_context) = dhcp_context {
            spawner.must_spawn(dhcp(context, dhcp_context));
        }

        for id in 0..WORKERS {
            spawner.must_spawn(http(id, context, api));
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
    async fn http(id: usize, context: &'static Context, api: &'static Api) -> ! {
        vertx_server::tasks::http(id, context, api).await
    }
}
