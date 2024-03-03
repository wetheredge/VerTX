use std::process;

use picoserve::Config;
use rand::Rng;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio::time::Duration;
use tokio::{task, time};
use vhs_api::{Request, Response};

#[derive(Debug)]
struct State {
    responses: Mutex<mpsc::Receiver<Response>>,
}

impl State {
    fn new(responses: mpsc::Receiver<Response>) -> Self {
        Self {
            responses: Mutex::new(responses),
        }
    }
}

impl vhs_api::State for State {
    fn handle_request(&self, request: Request) -> Option<Response> {
        match request {
            Request::ProtocolVersion => todo!(),
            Request::BuildInfo => {
                return Some(Response::BuildInfo {
                    major: 0,
                    minor: 0,
                    patch: 0,
                    suffix: "",
                    debug: true,
                    git_branch: "main",
                    git_commit: "0000000",
                    git_dirty: true,
                });
            }
            Request::PowerOff => {
                println!("Powering off");
                process::exit(0)
            }
            Request::Reboot => {
                println!("Rebooting");
                process::exit(0);
            }
            Request::CheckForUpdate => todo!(),
            Request::StreamInputs => todo!(),
            Request::StreamMixer => todo!(),
        }

        None
    }

    async fn next_response(&self) -> vhs_api::Response {
        (self.responses.lock().await.recv().await).unwrap()
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
    let addr = "localhost:8080";
    let (response_tx, response_rx) = mpsc::channel(10);

    let _ = task::LocalSet::new()
        .run_until(async {
            let status = task::spawn_local({
                let response_tx = response_tx.clone();
                async move {
                    let mut interval = time::interval(Duration::from_secs(10));

                    loop {
                        interval.tick().await;
                        response_tx
                            .send(Response::Status {
                                battery_voltage: 390,
                                idle_time: rand::thread_rng().gen_range(0.1..1.0),
                                timing_drift: rand::thread_rng().gen_range(-0.01..=0.01),
                            })
                            .await
                            .unwrap();
                    }
                }
            });

            let serve = task::spawn_local(async move {
                let router = picoserve::Router::new().route("/ws", vhs_api::UpgradeHandler);

                let state = State::new(response_rx);
                let socket = TcpListener::bind(addr).await.unwrap();
                println!("Listening on http://{addr}/");
                loop {
                    let (stream, remote) = socket.accept().await.unwrap();
                    println!("Got connection from {remote}");

                    if let Err(err) = picoserve::serve_with_state(
                        &router,
                        &CONFIG,
                        &mut [0; 2048],
                        stream,
                        &state,
                    )
                    .await
                    {
                        eprintln!("Error: {err:?}");
                    }
                }
            });

            tokio::join!(status, serve)
        })
        .await;
}
