use embassy_executor::{task, Spawner};
use esp_hal::rng::Rng;
use static_cell::make_static;
use vertx_server::tasks::DhcpContext;

use crate::api::Api;

const WORKERS: usize = 8;

vertx_server::get_router!(get_router -> Router<Api>);
type Context = vertx_server::Context<vertx_network_esp::Driver>;

pub(crate) type Start = impl FnOnce(vertx_network::Config);
pub(crate) fn get_start(
    spawner: Spawner,
    rng: Rng,
    network: vertx_network_esp::Hal,
    api: &'static Api,
) -> Start {
    move |config| {
        spawner.must_spawn(run(spawner, config, rng, network, api));
    }
}

#[task]
async fn run(
    spawner: Spawner,
    config: vertx_network::Config,
    rng: Rng,
    hal: vertx_network_esp::Hal,
    api: &'static Api,
) {
    let seed = get_seed(rng);

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

#[allow(clippy::host_endian_bytes)]
fn get_seed(mut rng: Rng) -> u64 {
    let mut seed = [0; 8];
    rng.read(&mut seed);
    u64::from_ne_bytes(seed)
}
