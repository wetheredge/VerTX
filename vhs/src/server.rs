use embassy_executor::{task, Spawner};
use embassy_net::tcp::TcpSocket;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Duration;
use picoserve::routing::{get, PathRouter};
use picoserve::{self, Config};
use static_cell::make_static;
use vhs_api::response;

pub const TASKS: usize = 8;
const TCP_BUFFER: usize = 1024;
const HTTP_BUFFER: usize = 2048;

include!(concat!(env!("OUT_DIR"), "/router.rs"));

pub type StatusSignal = Signal<CriticalSectionRawMutex, response::Status>;

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
    mode: crate::mode::Publisher<'static>,
    status: &'static StatusSignal,
) {
    let app = make_static!(router());
    let state = make_static!(State { status });

    let mut mode = Some(mode);
    for id in 0..TASKS {
        spawner.must_spawn(worker(id, stack, app, &CONFIG, state, mode.take()));
    }
}

#[task(pool_size = TASKS)]
async fn worker(
    id: usize,
    stack: &'static crate::wifi::Stack<'static>,
    router: &'static Router,
    config: &'static Config<Duration>,
    state: &'static State,
    mode: Option<crate::mode::Publisher<'static>>,
) -> ! {
    stack.wait_config_up().await;

    if let Some(mode) = mode {
        mode.publish(crate::Mode::WiFi);
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
    status: &'static StatusSignal,
}

impl vhs_api::State for State {
    const BUILD_INFO: response::BuildInfo = include!(concat!(env!("OUT_DIR"), "/build_info.rs"));

    async fn status(&self) -> response::Status {
        self.status.wait().await
    }

    fn power_off(&self) -> ! {
        todo!()
    }

    fn reboot(&self) -> ! {
        todo!()
    }
}
