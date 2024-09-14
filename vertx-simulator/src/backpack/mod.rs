mod network;

use std::sync::Arc;

use postcard::accumulator::{CobsAccumulator, FeedResult};
use tokio::sync::{mpsc, Mutex};
use tokio::task::AbortHandle;
use tokio_util::sync::CancellationToken;
use vertx_backpack_ipc::{ToBackpack, ToMain, INIT};
use vertx_simulator_ipc as ipc;

type Tx = mpsc::UnboundedSender<ipc::Message<'static, ipc::ToVertx>>;

#[derive(Debug)]
pub(crate) struct Backpack {
    rx_abort: Option<AbortHandle>,
}

impl Backpack {
    pub(crate) fn new() -> Self {
        Self { rx_abort: None }
    }

    pub(crate) fn start(&mut self, mut tx: Tx, rx: mpsc::UnboundedReceiver<Vec<u8>>) {
        self.stop();

        send_raw(&mut tx, &INIT);
        // Unused boot mode
        send_raw(&mut tx, &[0]);

        self.rx_abort = Some(tokio::spawn(handle_rx(tx, rx)).abort_handle());
    }

    fn stop(&mut self) {
        if let Some(rx) = self.rx_abort.take() {
            rx.abort();
        }
    }
}

impl Drop for Backpack {
    fn drop(&mut self) {
        if let Some(rx) = &self.rx_abort {
            rx.abort();
        }
    }
}

async fn handle_rx(mut tx: Tx, mut rx: mpsc::UnboundedReceiver<Vec<u8>>) {
    let tx = &mut tx;
    let (ws_tx, ws_rx) = mpsc::unbounded_channel();
    let ws_rx = Arc::new(Mutex::new(ws_rx));

    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let mut cancel = Some(cancel);

    let mut accumulator = CobsAccumulator::<256>::new();
    while let Some(buffer) = rx.recv().await {
        let mut chunk = &buffer[..];
        while !chunk.is_empty() {
            chunk = match accumulator.feed(chunk) {
                FeedResult::Consumed => break,
                FeedResult::OverFull(remaining) | FeedResult::DeserError(remaining) => remaining,
                FeedResult::Success { data, remaining } => {
                    match data {
                        ToBackpack::SetBootMode(_) => {
                            unimplemented!("simulator backpack does not handle boot mode")
                        }
                        ToBackpack::StartNetwork(_) => {
                            if let Some(cancel) = cancel.take() {
                                tokio::spawn(network::start(
                                    cancel,
                                    tx.clone(),
                                    Arc::clone(&ws_rx),
                                ));
                            } else {
                                eprintln!("Ignoring duplicate StartNetwork");
                            }
                        }
                        ToBackpack::ApiResponse(response) => ws_tx.send(response).unwrap(),
                        ToBackpack::ShutDown | ToBackpack::Reboot => send(tx, ToMain::PowerAck),
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

fn send_raw(tx: &mut Tx, message: &'static [u8]) {
    tx.send(ipc::Message::Backpack(message.into())).unwrap();
}
