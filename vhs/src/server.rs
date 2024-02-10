use embassy_executor::{task, Spawner};
use embassy_net::tcp::TcpSocket;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel};
use static_cell::make_static;
use vhs_server::api::{Request, Response};
use vhs_server::picoserve;

pub const TASKS: usize = 8;
const TCP_BUFFER: usize = 1024;
const HTTP_BUFFER: usize = 2048;
const RESPONSE_CHANNEL_SIZE: usize = 10;

pub type ApiResponseChannel =
    channel::Channel<CriticalSectionRawMutex, Response, RESPONSE_CHANNEL_SIZE>;
type ApiResponseReceiver<'ch> =
    channel::Receiver<'ch, CriticalSectionRawMutex, Response, RESPONSE_CHANNEL_SIZE>;

type Stack<'d> = embassy_net::Stack<esp_wifi::wifi::WifiDevice<'d, esp_wifi::wifi::WifiStaDevice>>;

type Router = picoserve::Router<impl picoserve::routing::PathRouter<State>, State>;
fn router() -> Router {
    vhs_server::router::<State>().route("/update", crate::ota::HttpHandler)
}

pub fn run(
    spawner: &Spawner,
    stack: &'static Stack<'static>,
    status: crate::status::Publisher<'static>,
    responses: ApiResponseReceiver<'static>,
) {
    let app = make_static!(router());
    let config = make_static!(vhs_server::CONFIG);
    let state = make_static!(State::new(responses));

    let mut status = Some(status);
    for id in 0..TASKS {
        spawner.must_spawn(worker(id, stack, app, config, state, status.take()));
    }
}

#[task(pool_size = TASKS)]
async fn worker(
    id: usize,
    stack: &'static Stack<'static>,
    router: &'static Router,
    config: &'static vhs_server::Config,
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

        if let Err(err) = vhs_server::serve::<HTTP_BUFFER, _>(router, config, tcp, state).await {
            log::error!("server({id}): Error: {err:?}");
        }
    }
}

// mod build_info {
//     // include!(concat!(env!("OUT_DIR"), "/shadow.rs"));
//     include!(concat!(env!("OUT_DIR"), "/build_info.rs"));
//
//     pub(super) const RESPONSE: super::Response = super::Response::BuildInfo {
//         major: VERSION_MAJOR,
//         minor: VERSION_MINOR,
//         patch: VERSION_PATCH,
//         suffix: VERSION_SUFFIX,
//         official: false,
//         debug: cfg!(debug_assertions),
//         date: env!("VERGEN_BUILD_TIMESTAMP"),
//         git_branch: env!("VERGEN_GIT_BRANCH"),
//         git_commit: env!("VERGEN_GIT_SHA"),
//         git_dirty: true,
//         rustc: "",
//     };
// }

struct State {
    responses: ApiResponseReceiver<'static>,
}

impl State {
    fn new(responses: ApiResponseReceiver<'static>) -> Self {
        Self { responses }
    }
}

impl vhs_server::State for State {
    fn handle_request(
        &self,
        request: vhs_server::api::Request,
    ) -> Option<vhs_server::api::Response> {
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

    async fn next_response(&self) -> vhs_server::api::Response {
        self.responses.receive().await
    }
}

mod build_info {}
