use std::process;

use picoserve::Config;
use rand::Rng;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio::time::Duration;
use tokio::{task, time};
use tracing_subscriber::prelude::*;
use vertx_api::{response, ApiState};

#[derive(Debug)]
struct State {
    api_state: ApiState,
    status: Mutex<mpsc::Receiver<response::Status>>,
    inputs: Mutex<mpsc::Receiver<Vec<u16>>>,
    outputs: Mutex<mpsc::Receiver<[u16; 16]>>,
}

impl State {
    fn new(
        status: mpsc::Receiver<response::Status>,
        inputs: mpsc::Receiver<Vec<u16>>,
        outputs: mpsc::Receiver<[u16; 16]>,
    ) -> Self {
        Self {
            api_state: ApiState::new(),
            status: Mutex::new(status),
            inputs: Mutex::new(inputs),
            outputs: Mutex::new(outputs),
        }
    }
}

impl vertx_api::State for State {
    const BUILD_INFO: response::BuildInfo = response::BuildInfo {
        target: "dev-server",
        major: 0,
        minor: 0,
        patch: 0,
        suffix: "local",
        debug: true,
        git_branch: "main",
        git_commit: "0000000",
        git_dirty: true,
    };

    fn api_state(&self) -> &ApiState {
        &self.api_state
    }

    async fn status(&self) -> response::Status {
        self.status.lock().await.recv().await.unwrap()
    }

    async fn inputs(&self) -> Vec<u16> {
        self.inputs.lock().await.recv().await.unwrap()
    }

    async fn outputs(&self) -> [u16; 16] {
        self.outputs.lock().await.recv().await.unwrap()
    }

    fn power_off(&self) -> ! {
        log::info!("Powering off");
        process::exit(0)
    }

    fn reboot(&self) -> ! {
        log::info!("Rebooting");
        process::exit(0)
    }
}

static CONFIG: Config<Duration> = picoserve::Config {
    timeouts: picoserve::Timeouts {
        start_read_request: Some(Duration::from_secs(5)),
        read_request: Some(Duration::from_secs(1)),
        write: Some(Duration::from_secs(1)),
    },
    connection: picoserve::KeepAlive::KeepAlive,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_filter(tracing_subscriber::filter::filter_fn(|_| true)),
        )
        .init();

    let addr = "localhost:8080";

    let _ = task::LocalSet::new()
        .run_until(async {
            let (status_rx, status) = send_periodically(
                response::Status {
                    battery_voltage: 390,
                    idle_time: rand::thread_rng().gen_range(0.1..1.0),
                    timing_drift: rand::thread_rng().gen_range(-0.01..=0.01),
                },
                Duration::from_secs(10),
            );
            let (inputs_rx, inputs) = send_periodically(vec![], Duration::from_secs(1));
            let (outputs_rx, outputs) =
                send_periodically(std::array::from_fn(|_| 0), Duration::from_secs(1));

            let serve = task::spawn_local(async move {
                let router = picoserve::Router::new().route("/ws", vertx_api::UpgradeHandler);

                let state = State::new(status_rx, inputs_rx, outputs_rx);
                let socket = TcpListener::bind(addr).await.unwrap();
                log::info!("Listening on http://{addr}/");
                loop {
                    let (stream, remote) = socket.accept().await.unwrap();
                    log::debug!("Got connection from {remote}");

                    if let Err(err) = picoserve::serve_with_state(
                        &router,
                        &CONFIG,
                        &mut [0; 2048],
                        stream,
                        &state,
                    )
                    .await
                    {
                        log::error!("Error: {err:?}");
                    }
                }
            });

            tokio::join!(serve, status, inputs, outputs)
        })
        .await;
}

fn send_periodically<T: Clone + 'static>(
    item: T,
    period: Duration,
) -> (mpsc::Receiver<T>, task::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(1);
    let mut interval = time::interval(period);

    let handle = task::spawn_local(async move {
        loop {
            interval.tick().await;
            tx.send(item.clone()).await.unwrap();
        }
    });

    (rx, handle)
}
