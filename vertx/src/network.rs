use core::net::Ipv4Addr;

use embassy_executor::Spawner;

pub enum Error {
    InvalidHomeConfig,
}

const STATIC_ADDRESS: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 1);

pub async fn run(
    spawner: Spawner,
    is_home: bool,
    config: crate::Config,
    api: &'static crate::api::Api,
    #[cfg(feature = "network-native")] mut network: crate::hal::Network,
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
    {
        use crate::hal::traits::Network;
        native::run(spawner, server_config, network.seed(), network.hal(), api).await;
    }
    #[cfg(not(feature = "network-native"))]
    let _ = spawner;

    #[cfg(feature = "network-backpack")]
    backpack.start_network(server_config, api).await;

    Ok(())
}

#[cfg(feature = "network-native")]
mod native {
    use embassy_executor::{task, Spawner};
    use static_cell::ConstStaticCell;
    use vertx_server::tasks::DhcpContext;
    use vertx_server::Stack;

    use crate::api::Api;

    const WORKERS: usize = 8;

    pub(super) async fn run(
        spawner: Spawner,
        config: vertx_network::Config,
        seed: u64,
        hal: crate::hal::NetworkHal,
        api: &'static Api,
    ) {
        static RESOURCES: ConstStaticCell<vertx_server::Resources<{ WORKERS + 2 }>> =
            ConstStaticCell::new(vertx_server::Resources::new());
        let (stack, runner, dhcp_context) = vertx_server::init(RESOURCES.take(), config, seed, hal);

        spawner.must_spawn(network(runner));

        if let Some(dhcp_context) = dhcp_context {
            spawner.must_spawn(dhcp(stack, dhcp_context));
        }

        for id in 0..WORKERS {
            spawner.must_spawn(http(id, stack, api));
        }

        stack.wait_link_up().await;
    }

    #[task]
    async fn network(mut runner: vertx_server::Runner<'static, crate::hal::NetworkDriver>) -> ! {
        runner.run().await
    }

    #[task]
    async fn dhcp(stack: Stack<'static>, dhcp: DhcpContext) -> ! {
        vertx_server::tasks::dhcp(stack, dhcp).await
    }

    #[task(pool_size = WORKERS)]
    async fn http(id: usize, stack: Stack<'static>, api: &'static Api) -> ! {
        vertx_server::tasks::http(id, stack, api).await
    }
}
