use postcard::accumulator::{CobsAccumulator, FeedResult};
use tokio::sync::mpsc;
use vertx_backpack_ipc::{ToBackpack, ToMain};
use vertx_simulator_ipc as ipc;

type Tx = mpsc::UnboundedSender<ipc::Message<'static, ipc::ToVertx>>;

#[derive(Debug)]
pub(crate) struct Backpack {
    rx_abort: Option<tokio::task::AbortHandle>,
}

impl Backpack {
    pub(crate) fn new() -> Self {
        Self { rx_abort: None }
    }

    pub(crate) fn start(&mut self, mut tx: Tx, rx: mpsc::UnboundedReceiver<Vec<u8>>) {
        self.stop();

        let init = ToMain::Init { boot_mode: 0 };
        send(&mut tx, init);

        self.rx_abort = Some(tokio::spawn(handle_rx(tx, rx)).abort_handle());
    }

    fn stop(&mut self) {
        if let Some(rx) = self.rx_abort.take() {
            rx.abort();
        }
    }
}

async fn handle_rx(mut tx: Tx, mut rx: mpsc::UnboundedReceiver<Vec<u8>>) {
    let tx = &mut tx;

    let mut accumulator = CobsAccumulator::<256>::new();
    while let Some(buffer) = rx.recv().await {
        let mut chunk = &buffer[..];
        while !chunk.is_empty() {
            chunk = match accumulator.feed(chunk) {
                FeedResult::Consumed => break,
                FeedResult::OverFull(remaining) | FeedResult::DeserError(remaining) => remaining,
                FeedResult::Success { data, remaining } => {
                    tx.send(ipc::Message::Simulator(ipc::ToVertx::BackpackAck))
                        .unwrap();

                    match data {
                        ToBackpack::InitAck => {}
                        ToBackpack::SetBootMode(_) => {
                            unimplemented!("simulator backpack does not handle boot mode")
                        }
                        ToBackpack::StartNetwork(config) => {
                            dbg!(config);
                            todo!("start network")
                        }
                        ToBackpack::ApiResponse(response) => {
                            dbg!(response);
                            todo!("respond")
                        }
                        ToBackpack::Reboot => {
                            todo!("reboot backpack")
                        }
                    }

                    remaining
                }
            }
        }
    }
}

fn send(tx: &mut Tx, message: ToMain) {
    let message = postcard::to_stdvec_cobs(&message).unwrap();
    tx.send(ipc::Message::Backpack(message.into())).unwrap();
}
