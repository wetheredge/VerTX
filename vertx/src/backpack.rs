use embassy_executor::{task, Spawner};
use embassy_sync::channel;
use embassy_sync::once_lock::OnceLock;
use embassy_sync::signal::Signal;
use embedded_io_async::{Read, Write};
use portable_atomic::{AtomicUsize, Ordering};
use postcard::accumulator::{CobsAccumulator, FeedResult};
use static_cell::make_static;
use vertx_backpack_ipc::{ToBackpack, ToMain};
use vertx_network::Api as _;

use crate::api::Api;
use crate::hal::traits::BackpackAck as _;

type TxChannel = channel::Channel<crate::mutex::SingleCore, ToBackpack, 10>;
type TxSender = channel::Sender<'static, crate::mutex::SingleCore, ToBackpack, 10>;
type TxReceiver = channel::Receiver<'static, crate::mutex::SingleCore, ToBackpack, 10>;

type AckSignal = Signal<crate::mutex::SingleCore, ()>;

#[cfg(feature = "network-backpack")]
static API: OnceLock<&'static Api> = OnceLock::new();
#[cfg(feature = "backpack-boot-mode")]
static BOOT_MODE: OnceLock<u8> = OnceLock::new();
static SENT: AtomicUsize = AtomicUsize::new(0);
static ACKED: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct Backpack {
    tx: TxSender,
    ack: &'static AckSignal,
}

impl Backpack {
    pub(crate) fn new(spawner: Spawner, backpack: crate::hal::Backpack) -> Self {
        let channel = make_static!(TxChannel::new());
        let ack_signal = make_static!(AckSignal::new());

        spawner.must_spawn(tx_handler(backpack.tx, channel.receiver()));
        spawner.must_spawn(rx_handler(
            spawner,
            backpack.rx,
            channel.sender(),
            backpack.ack,
            ack_signal,
        ));

        Self {
            tx: channel.sender(),
            ack: ack_signal,
        }
    }

    #[cfg(feature = "backpack-boot-mode")]
    pub(crate) async fn boot_mode(&self) -> crate::BootMode {
        (*BOOT_MODE.get().await).into()
    }

    #[cfg(feature = "backpack-boot-mode")]
    pub(crate) async fn set_boot_mode(&self, mode: u8) {
        self.send(ToBackpack::SetBootMode(mode)).await;
    }

    pub(crate) async fn reboot(&self) {
        self.send(ToBackpack::Reboot).await;
    }

    #[cfg(feature = "network-backpack")]
    pub(crate) async fn start_network(&self, config: vertx_network::Config, api: &'static Api) {
        API.init(api).map_err(|_| ()).unwrap();
        self.send(ToBackpack::StartNetwork(config)).await;
    }

    async fn send(&self, message: ToBackpack) {
        let id = SENT.fetch_add(1, Ordering::Relaxed);
        loog::info!("backpack tx: {message:?}");
        self.tx.send(message).await;

        while ACKED.load(Ordering::Relaxed) < id {
            self.ack.wait().await;
        }
    }
}

#[task]
async fn tx_handler(mut tx: crate::hal::BackpackTx, messages: TxReceiver) -> ! {
    let mut buffer = [0; 256];

    loop {
        let message = messages.receive().await;
        let bytes = match postcard::to_slice_cobs(&message, &mut buffer) {
            Ok(bytes) => bytes,
            Err(err) => {
                loog::error!("Failed to serialize message to backpack: {err:?}");
                continue;
            }
        };

        match tx.write_all(bytes).await {
            Ok(()) => {}
            Err(err) => {
                loog::error!("Failed to send message to backpack: {err:?}");
            }
        }
    }
}

#[task]
async fn rx_handler(
    spawner: Spawner,
    mut rx: crate::hal::BackpackRx,
    tx: TxSender,
    ack: crate::hal::BackpackAck,
    ack_signal: &'static AckSignal,
) {
    let mut spawn_ack_handler = Some(|| spawner.must_spawn(ack_handler(ack, ack_signal)));
    let mut api_buffer = Api::buffer();

    let mut ever_success = false;
    let mut buffer = [0; 32];
    let mut accumulator = CobsAccumulator::<256>::new();
    loop {
        let mut chunk = match rx.read(&mut buffer).await {
            Ok(len) => &buffer[0..len],
            Err(err) => {
                loog::error!("Backpack rx failed: {err:?}");
                continue;
            }
        };

        while !chunk.is_empty() {
            chunk = match accumulator.feed(chunk) {
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
                    match data {
                        #[allow(unused)]
                        ToMain::Init { boot_mode } => {
                            spawn_ack_handler.take().unwrap()();

                            #[cfg(feature = "backpack-boot-mode")]
                            BOOT_MODE.init(boot_mode).unwrap();
                            tx.send(ToBackpack::InitAck).await;
                        }

                        #[cfg(feature = "network-backpack")]
                        ToMain::NetworkUp => loog::info!("Network started"),
                        #[cfg(not(feature = "network-backpack"))]
                        ToMain::NetworkUp => {
                            loog::error!("Ignoring NetworkUp message from backpack")
                        }

                        #[cfg(feature = "network-backpack")]
                        ToMain::ApiRequest(request) => {
                            if let Some(api) = API.try_get() {
                                if let Some(response) = api.handle(&request, &mut api_buffer).await
                                {
                                    tx.send(ToBackpack::ApiResponse(response.to_vec())).await;
                                }
                            } else {
                                loog::error!("Got ApiRequest before network was initialized");
                            }
                        }
                        #[cfg(not(feature = "network-backpack"))]
                        ToMain::ApiRequest(_) => {
                            loog::error!("Ignoring ApiRequest message from backpack")
                        }
                    }

                    remaining
                }
            }
        }
    }
}

#[task]
async fn ack_handler(mut ack: crate::hal::BackpackAck, signal: &'static AckSignal) -> ! {
    loop {
        ack.wait().await;
        ACKED.add(1, Ordering::Relaxed);
        signal.signal(());
    }
}
