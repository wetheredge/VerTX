use embassy_executor::{Spawner, task};
use esp_hal::rng::Rng;
use static_cell::ConstStaticCell;
use vertx_server::Stack;
use vertx_server::tasks::DhcpContext;

use crate::api::Api;

const WORKERS: usize = 8;

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
    ipc.send_network_up().await;
}

#[task]
async fn network(mut runner: vertx_server::Runner<'static, vertx_network_esp::Driver>) -> ! {
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

#[expect(clippy::host_endian_bytes)]
fn get_seed(mut rng: Rng) -> u64 {
    let mut seed = [0; 8];
    rng.read(&mut seed);
    u64::from_ne_bytes(seed)
}
