use alloc::vec::Vec;

use embassy_executor::task;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use esp_hal::uart::{Uart, UartRx, UartTx};
use postcard::accumulator::{CobsAccumulator, FeedResult};
use vertx_backpack_ipc::{INIT, ToBackpack, ToMain};

pub(crate) struct Context {
    messages: Channel<NoopRawMutex, ToMain, 10>,
    flushed: Signal<NoopRawMutex, ()>,
}

impl Context {
    pub(crate) fn send_network_up(&self) -> impl Future {
        self.messages.send(ToMain::NetworkUp)
    }

    pub(crate) fn send_api_request(&self, request: Vec<u8>) -> impl Future {
        self.messages.send(ToMain::ApiRequest(request))
    }
}

pub(crate) async fn init(uart: &mut Uart<'static, esp_hal::Async>) -> Context {
    loop {
        loog::info!("Waiting for init");
        let mut byte = [0];
        if uart.read_async(&mut byte).await.is_err() {
            continue;
        }

        if byte[0] == INIT[0] {
            let mut init = [0; { INIT.len() - 1 }];
            let Ok(len) = uart.read_async(&mut init).await else {
                continue;
            };

            if len == init.len() && init == INIT[1..] {
                break;
            }
        }
    }

    // Assume all bytes were written, since the message is quite short
    let _ = uart.write_async(&INIT).await.unwrap();

    loog::info!("Init finished");

    Context {
        messages: Channel::new(),
        flushed: Signal::new(),
    }
}

#[task]
pub(crate) async fn tx(mut tx: UartTx<'static, esp_hal::Async>, context: &'static Context) -> ! {
    let mut buffer = [0; 256];

    loop {
        let message = context.messages.receive().await;
        loog::debug!("Backpack tx: {message:?}");
        let flush = matches!(message, ToMain::PowerAck);

        let bytes = match postcard::to_slice_cobs(&message, &mut buffer) {
            Ok(bytes) => bytes,
            Err(err) => {
                loog::error!("Failed to serialize message: {err:?}");
                continue;
            }
        };

        match tx.write_async(bytes).await {
            Ok(_) => {}
            Err(err) => {
                loog::error!("Failed to send message: {err:?}");
            }
        }

        if flush {
            tx.flush_async().await.unwrap();
            context.flushed.signal(());
        }
    }
}

#[task]
pub(crate) async fn rx(
    mut rx: UartRx<'static, esp_hal::Async>,
    start_network: crate::network::Start,
    api: &'static crate::Api,
    context: &'static Context,
) -> ! {
    let mut start_network = Some(start_network);

    let mut ever_success = false;
    let mut buffer = [0; 32];
    let mut accumulator = CobsAccumulator::<256>::new();
    loop {
        let mut chunk = match rx.read_async(&mut buffer).await {
            Ok(len) => &buffer[0..len],
            Err(err) => {
                loog::error!("Backpack rx failed: {err:?}");
                continue;
            }
        };

        while !chunk.is_empty() {
            chunk = match accumulator.feed_ref(chunk) {
                FeedResult::Consumed => break,
                FeedResult::OverFull(remaining) => remaining,
                FeedResult::DeserError(remaining) => {
                    if ever_success {
                        loog::error!("Backpack rx decode failed");
                    }
                    remaining
                }
                FeedResult::Success { data, remaining } => {
                    ever_success = true;
                    loog::debug!("Backpack rx: {data:?}");
                    match data {
                        ToBackpack::StartNetwork(config) => {
                            if let Some(start) = start_network.take() {
                                start(config);
                            } else {
                                loog::warn!("Network already started");
                            }
                        }

                        ToBackpack::ApiResponse(response) => {
                            api.push_api_response(response).await;
                        }

                        ToBackpack::ShutDown => {
                            context.messages.send(ToMain::PowerAck).await;
                            todo!()
                        }
                        ToBackpack::Reboot => {
                            context.flushed.reset();
                            context.messages.send(ToMain::PowerAck).await;
                            context.flushed.wait().await;
                            Timer::after_millis(1).await;
                            esp_hal::reset::software_reset();
                        }
                    }

                    remaining
                }
            }
        }
    }
}
