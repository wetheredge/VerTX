use embassy_executor::{task, Spawner};
use esp_hal::rng::Rng;
use static_cell::{ConstStaticCell, StaticCell};
use vertx_server::tasks::DhcpContext;

use crate::api::Api;

const WORKERS: usize = 8;

type Context = vertx_server::Context<vertx_network_esp::Driver>;

pub(crate) type Start = impl FnOnce(vertx_network::Config);
pub(crate) fn get_start(
    spawner: Spawner,
    rng: Rng,
    network: vertx_network_esp::Hal,
    api: &'static Api,
    ipc: &'static crate::ipc::Context,
) -> Start {
    move |config| {
        spawner.must_spawn(run(spawner, config, rng, network, api, ipc));
    }
}

#[task]
async fn run(
    spawner: Spawner,
    config: vertx_network::Config,
    rng: Rng,
    hal: vertx_network_esp::Hal,
    api: &'static Api,
    ipc: &'static crate::ipc::Context,
) {
    let seed = get_seed(rng);

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
    ipc.send_network_up().await;
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

#[allow(clippy::host_endian_bytes)]
fn get_seed(mut rng: Rng) -> u64 {
    let mut seed = [0; 8];
    rng.read(&mut seed);
    u64::from_ne_bytes(seed)
}
