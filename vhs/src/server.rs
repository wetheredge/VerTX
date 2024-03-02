use embassy_executor::{task, Spawner};
use embassy_net::tcp::TcpSocket;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel;
use embassy_time::Duration;
use picoserve::routing::{get, PathRouter};
use picoserve::{self, Config};
use static_cell::make_static;
use vhs_api::{Request, Response};

pub const TASKS: usize = 8;
const TCP_BUFFER: usize = 1024;
const HTTP_BUFFER: usize = 2048;
const RESPONSE_CHANNEL_SIZE: usize = 10;

include!(concat!(env!("OUT_DIR"), "/router.rs"));

pub type ApiResponseChannel =
    channel::Channel<CriticalSectionRawMutex, Response, RESPONSE_CHANNEL_SIZE>;
type ApiResponseReceiver<'ch> =
    channel::Receiver<'ch, CriticalSectionRawMutex, Response, RESPONSE_CHANNEL_SIZE>;

type Router = picoserve::Router<impl PathRouter<State>, State>;
fn router() -> Router {
    router! {
        "/update" => crate::ota::HttpHandler
        "/ws" => vhs_api::UpgradeHandler
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

pub fn run(
    spawner: &Spawner,
    stack: &'static crate::wifi::Stack<'static>,
    status: crate::status::Publisher<'static>,
    responses: ApiResponseReceiver<'static>,
) {
    let app = make_static!(router());
    let state = make_static!(State::new(responses));

    let mut status = Some(status);
    for id in 0..TASKS {
        spawner.must_spawn(worker(id, stack, app, &CONFIG, state, status.take()));
    }
}

#[task(pool_size = TASKS)]
async fn worker(
    id: usize,
    stack: &'static crate::wifi::Stack<'static>,
    router: &'static Router,
    config: &'static Config<Duration>,
    state: &'static State,
    status: Option<crate::status::Publisher<'static>>,
) -> ! {
    stack.wait_config_up().await;

    if let Some(status) = status {
        status.publish(crate::Status::WiFi);
    }

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
    responses: ApiResponseReceiver<'static>,
}

impl State {
    fn new(responses: ApiResponseReceiver<'static>) -> Self {
        Self { responses }
    }
}

impl vhs_api::State for State {
    fn handle_request(&self, request: vhs_api::Request) -> Option<vhs_api::Response> {
        match request {
            Request::ProtocolVersion => return Some(Response::protocol_version()),
            Request::BuildInfo => {
                return Some(include!(concat!(env!("OUT_DIR"), "/build_info.rs")));
            }
            Request::PowerOff => todo!(),
            Request::Reboot => todo!(),
            Request::CheckForUpdate => todo!(),
            Request::StreamInputs => todo!(),
            Request::StreamMixer => todo!(),
        }

        None
    }

    async fn next_response(&self) -> vhs_api::Response {
        self.responses.receive().await
    }
}

mod build_info {}
