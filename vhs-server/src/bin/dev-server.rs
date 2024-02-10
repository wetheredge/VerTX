use std::process;

use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::{net::TcpListener, sync::Mutex};
use tokio::{task, time};
use vhs_server::api::{Request, Response};

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

impl vhs_server::State for State {
    fn handle_request(&self, request: Request) -> Option<Response> {
        match request {
            Request::ProtocolVersion => todo!(),
            Request::BuildInfo => {
                return Some(Response::BuildInfo {
                    major: 0,
                    minor: 0,
                    patch: 0,
                    commit: "abcdef",
                    dirty: true,
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

    async fn next_response(&self) -> vhs_server::api::Response {
        (self.responses.lock().await.recv().await).unwrap()
    }
}

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
                            })
                            .await
                            .unwrap();
                    }
                }
            });

            let serve = task::spawn_local(async move {
                let router = vhs_server::router::<State>();
                let config = vhs_server::CONFIG;
                let state = State::new(response_rx);
                let socket = TcpListener::bind(addr).await.unwrap();
                println!("Listening on http://{addr}/");
                loop {
                    let (stream, remote) = socket.accept().await.unwrap();
                    println!("Got connection from {remote}");

                    if let Err(err) =
                        vhs_server::serve::<2048, _>(&router, &config, stream, &state).await
                    {
                        eprintln!("Error: {err:?}");
                    }
                }
            });

            tokio::join!(status, serve)
        })
        .await;
}
