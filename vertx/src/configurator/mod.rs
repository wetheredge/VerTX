mod api;
pub(crate) mod protocol;

use embassy_executor::{task, Spawner};
use embassy_net::tcp::TcpSocket;
use embassy_net::Stack;
use embassy_sync::signal::Signal;
use embassy_time::Duration;
use picoserve::response::ResponseWriter;
use picoserve::routing::{PathRouter, PathRouterService, RequestHandlerService};
use picoserve::{self, Config, ResponseSent};
use static_cell::make_static;

use self::protocol::response;

pub const TASKS: usize = 8;
const TCP_BUFFER: usize = 1024;
const HTTP_BUFFER: usize = 2048;

pub type StatusSignal = Signal<crate::mutex::SingleCore, response::Status>;

include!(concat!(env!("OUT_DIR"), "/configurator.rs"));

type Router = picoserve::Router<impl PathRouter<State>, State>;
fn get_router() -> Router {
    picoserve::Router::new()
        .nest_service("", AssetsRouter)
        .nest_service("/api", api::UpgradeHandler)
}

struct AssetsRouter;
impl PathRouterService<State> for AssetsRouter {
    async fn call_request_handler_service<
        R: picoserve::io::Read,
        W: ResponseWriter<Error = R::Error>,
    >(
        &self,
        state: &State,
        _current_path_parameters: (),
        path: picoserve::request::Path<'_>,
        request: picoserve::request::Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        let path = path.encoded();
        let file = if let Ok(asset) = ASSETS.binary_search_by_key(&path, |(route, _)| route) {
            &ASSETS[asset].1
        } else {
            &INDEX
        };
        file.call_request_handler_service(state, (), request, response_writer)
            .await
    }
}

static CONFIG: Config<Duration> = Config {
    timeouts: picoserve::Timeouts {
        start_read_request: Some(Duration::from_secs(5)),
        read_request: Some(Duration::from_secs(1)),
        write: Some(Duration::from_secs(2)),
    },
    connection: picoserve::KeepAlive::KeepAlive,
};

#[task]
pub async fn run(
    spawner: Spawner,
    config: &'static crate::config::Manager,
    stack: &'static Stack<crate::hal::Wifi>,
    stack_ready: crate::wifi::StackReady,
    mode: crate::mode::Publisher<'static>,
    status: &'static StatusSignal,
) {
    let router = make_static!(get_router());
    let state = make_static!(State { config, status });

    stack_ready.await;
    mode.publish(crate::Mode::Configurator);

    for id in 0..TASKS {
        spawner.must_spawn(worker(id, stack, router, &CONFIG, state));
    }
}

#[task(pool_size = TASKS)]
async fn worker(
    id: usize,
    stack: &'static Stack<crate::hal::Wifi>,
    router: &'static Router,
    config: &'static Config<Duration>,
    state: &'static State,
) -> ! {
    let mut rx_buffer = [0; TCP_BUFFER];
    let mut tx_buffer = [0; TCP_BUFFER];

    loop {
        let mut tcp = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

        if let Err(err) = tcp.accept(80).await {
            log::warn!("server({id}): Accept error: {err:?}");
            continue;
        }

        if let Err(err) =
            picoserve::serve_with_state(router, config, &mut [0; HTTP_BUFFER], tcp, state).await
        {
            log::error!("server({id}): Error: {err:?}");
        }
    }
}

struct State {
    config: &'static crate::config::Manager,
    status: &'static StatusSignal,
}
