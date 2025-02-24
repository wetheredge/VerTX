use core::net::Ipv4Addr;

use embassy_executor::Spawner;
use embassy_sync::signal::Signal;

const STATIC_ADDRESS: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 1);

pub(super) static START: Signal<crate::mutex::MultiCore, ()> = Signal::new();

pub(crate) fn start() {
    START.signal(());
}

pub async fn init(
    spawner: Spawner,
    config: crate::Config,
    api: &'static crate::api::Api,
    #[cfg(feature = "network-native")] mut network: crate::hal::Network,
    #[cfg(feature = "network-backpack")] backpack: crate::backpack::Backpack,
) {
    loog::info!("Starting network");

    let config = config.network().lock(|config| {
        let home = {
            let home = config.home();
            let ssid = home.ssid();
            let password = home.password();

            (!ssid.is_empty() && !password.is_empty()).then(|| vertx_network::HomeConfig {
                credentials: vertx_network::Credentials {
                    ssid: ssid.clone(),
                    password: password.clone(),
                },
                hostname: "vertx".try_into().unwrap(),
            })
        };

        let field = vertx_network::FieldConfig {
            credentials: vertx_network::Credentials {
                ssid: "VerTX".try_into().unwrap(),
                password: config.password().clone(),
            },
            address: STATIC_ADDRESS,
        };

        vertx_network::Config { home, field }
    });

    #[cfg(feature = "network-native")]
    {
        use crate::hal::traits::Network as _;
        native::run(spawner, config, network.seed(), network.hal(), api).await;
    }
    #[cfg(not(feature = "network-native"))]
    let _ = spawner;

    #[cfg(feature = "network-backpack")]
    backpack.start_network(config, api).await;
}

#[cfg(feature = "network-native")]
mod native {
    use embassy_executor::{Spawner, task};
    use static_cell::ConstStaticCell;
    use vertx_server::Stack;
    use vertx_server::tasks::DhcpContext;

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
        let (stack, runner, dhcp_context) =
            vertx_server::init(RESOURCES.take(), config, seed, hal).await;

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
