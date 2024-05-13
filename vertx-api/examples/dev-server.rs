use std::process;

use picoserve::Config;
use rand::Rng;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio::time::Duration;
use tokio::{task, time};
use tracing_subscriber::prelude::*;
use vertx_api::response;

#[derive(Debug)]
struct State {
    status: Mutex<mpsc::Receiver<response::Status>>,
}

impl State {
    fn new(status: mpsc::Receiver<response::Status>) -> Self {
        Self {
            status: Mutex::new(status),
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

    async fn status(&self) -> response::Status {
        (self.status.lock().await.recv().await).unwrap()
    }

    fn power_off(&self) {
        log::info!("Powering off");
        process::exit(0)
    }

    fn reboot(&self) {
        log::info!("Rebooting");
        process::exit(0)
    }

    fn exit_configurator(&self) {
        log::info!("Exiting configurator");
        process::exit(0)
    }

    fn config(&self) -> &impl vertx_config::Storage {
        #[derive(Default, vertx_config::Storage)]
        struct Config {}
        &Config {}
    }

    async fn update_config<'a>(
        &self,
        key: &'a str,
        update: vertx_config::update::Update<'a>,
    ) -> vertx_config::update::Result {
        log::info!("{key:?} = {update:?}");
        Ok(())
    }
}

static CONFIG: Config<Duration> = picoserve::Config {
    timeouts: picoserve::Timeouts {
        start_read_request: Some(Duration::from_secs(5)),
        read_request: Some(Duration::from_secs(1)),
        write: Some(Duration::from_secs(1)),
    },
    connection: picoserve::KeepAlive::KeepAlive,
    shutdown_method: picoserve::ShutdownMethod::Shutdown,
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
    let (status_tx, status_rx) = mpsc::channel(1);

    let set = task::LocalSet::new();

    set.spawn_local({
        let status_tx = status_tx.clone();
        async move {
            let mut interval = time::interval(Duration::from_secs(10));

            loop {
                interval.tick().await;
                status_tx
                    .send(response::Status {
                        battery_voltage: 390,
                        idle_time: rand::thread_rng().gen_range(0.1..1.0),
                        timing_drift: rand::thread_rng().gen_range(-0.01..=0.01),
                    })
                    .await
                    .unwrap();
            }
        }
    });

    set.spawn_local(async move {
        let router = picoserve::Router::new().nest_service("/api", vertx_api::UpgradeHandler);

        let state = State::new(status_rx);
        let socket = TcpListener::bind(addr).await.unwrap();
        log::info!("Listening on http://{addr}/");
        loop {
            let (stream, remote) = socket.accept().await.unwrap();
            log::debug!("Got connection from {remote}");

            if let Err(err) =
                picoserve::serve_with_state(&router, &CONFIG, &mut [0; 2048], stream, &state).await
            {
                log::error!("Error: {err:?}");
            }
        }
    });

    set.await;
}
