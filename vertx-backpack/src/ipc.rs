use core::sync::atomic::{AtomicBool, Ordering};

use embassy_executor::task;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel;
use esp_hal::uart::{UartRx, UartTx};
use esp_hal::{peripherals, Async};
use postcard::accumulator::{CobsAccumulator, FeedResult};
use vertx_backpack_ipc::{ToBackpack, ToMain};

pub(crate) type TxChannel = channel::Channel<NoopRawMutex, ToMain, 10>;
pub(crate) type TxSender = channel::Sender<'static, NoopRawMutex, ToMain, 10>;
pub(crate) type TxReceiver = channel::Receiver<'static, NoopRawMutex, ToMain, 10>;

#[task]
pub(crate) async fn tx(
    mut tx: UartTx<'static, peripherals::UART0, Async>,
    messages: TxReceiver,
) -> ! {
    let mut buffer = [0; 256];

    loop {
        let message = messages.receive().await;
        let bytes = match postcard::to_slice_cobs(&message, &mut buffer) {
            Ok(bytes) => bytes,
            Err(err) => {
                log::error!("Failed to serialize message: {err:?}");
                continue;
            }
        };

        match tx.write_async(bytes).await {
            Ok(_) => {}
            Err(err) => {
                log::error!("Failed to send message: {err:?}");
            }
        }
    }
}

#[task]
pub(crate) async fn rx(
    mut rx: UartRx<'static, peripherals::UART0, Async>,
    init_acked: &'static AtomicBool,
    start_network: crate::network::Start,
    api_responses: crate::api::ResponseSender,
) -> ! {
    let mut start_network = Some(start_network);

    let mut ever_success = false;
    let mut buffer = [0; 32];
    let mut accumulator = CobsAccumulator::<256>::new();
    loop {
        let mut chunk = match rx.read_async(&mut buffer).await {
            Ok(len) => &buffer[0..len],
            Err(err) => {
                log::error!("Backpack rx failed: {err:?}");
                continue;
            }
        };

        while !chunk.is_empty() {
            chunk = match accumulator.feed(chunk) {
                FeedResult::Consumed => break,
                FeedResult::OverFull(remaining) => remaining,
                FeedResult::DeserError(remaining) => {
                    if ever_success {
                        log::error!("Backpack rx decode failed");
                    }
                    remaining
                }
                FeedResult::Success { data, remaining } => {
                    ever_success = true;
                    match data {
                        ToBackpack::InitAck => {
                            if init_acked.swap(true, Ordering::Relaxed) {
                                log::warn!("Repeated InitAck")
                            }
                        }

                        ToBackpack::StartNetwork(config) => {
                            if let Some(start) = start_network.take() {
                                start(config)
                            } else {
                                log::warn!("Network already started")
                            }
                        }

                        ToBackpack::ApiResponse(response) => api_responses.send(response).await,
                    }

                    remaining
                }
            }
        }
    }
}
