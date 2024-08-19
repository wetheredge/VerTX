use embassy_executor::{task, Spawner};
use embassy_sync::channel;
use embassy_sync::once_lock::OnceLock;
use embassy_sync::signal::Signal;
use embedded_io_async::{Read, Write};
use portable_atomic::{AtomicUsize, Ordering};
use postcard::accumulator::{CobsAccumulator, FeedResult};
use rand::RngCore as _;
use static_cell::make_static;
use vertx_backpack_ipc::{ToBackpack, ToMain};
use vertx_network::Api as _;

use crate::api::Api;
use crate::hal::traits::Backpack as _;

type TxChannel = channel::Channel<crate::mutex::SingleCore, ToBackpack, 10>;
type TxSender = channel::Sender<'static, crate::mutex::SingleCore, ToBackpack, 10>;
type TxReceiver = channel::Receiver<'static, crate::mutex::SingleCore, ToBackpack, 10>;

#[cfg(feature = "backpack-boot-mode")]
pub(crate) type SetBootModeAck = Signal<crate::mutex::SingleCore, ()>;

#[cfg(feature = "network-backpack")]
static API: OnceLock<&'static Api> = OnceLock::new();
#[cfg(feature = "backpack-boot-mode")]
static BOOT_MODE: OnceLock<u8> = OnceLock::new();
static MESSAGE_ID: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct Backpack {
    tx: TxSender,
    #[cfg(feature = "backpack-boot-mode")]
    set_boot_mode_ack: &'static SetBootModeAck,
}

impl Backpack {
    pub(crate) fn new(
        spawner: Spawner,
        backpack: crate::hal::Backpack,
        rng: &mut crate::hal::Rng,
    ) -> Self {
        let channel = make_static!(TxChannel::new());
        #[cfg(feature = "backpack-boot-mode")]
        let set_boot_mode_ack = make_static!(Signal::new());

        #[cfg(target_pointer_width = "32")]
        let first_id = rng.next_u32();
        #[cfg(target_pointer_width = "64")]
        let first_id = rng.next_u64();
        MESSAGE_ID.store(first_id as usize, Ordering::Relaxed);

        let backpack = backpack.split();
        spawner.must_spawn(tx(backpack.0, channel.receiver()));
        spawner.must_spawn(rx(
            backpack.1,
            channel.sender(),
            #[cfg(feature = "backpack-boot-mode")]
            set_boot_mode_ack,
        ));

        Self {
            tx: channel.sender(),
            #[cfg(feature = "backpack-boot-mode")]
            set_boot_mode_ack,
        }
    }

    #[cfg(feature = "backpack-boot-mode")]
    pub(crate) async fn boot_mode(&self) -> crate::BootMode {
        (*BOOT_MODE.get().await).into()
    }

    #[cfg(feature = "backpack-boot-mode")]
    pub(crate) async fn set_boot_mode(&self, mode: u8) {
        self.set_boot_mode_ack.reset();
        self.tx.send(ToBackpack::SetBootMode(mode)).await;
        self.set_boot_mode_ack.wait().await;
    }

    pub(crate) async fn reboot(&self) {
        todo!()
    }

    pub(crate) async fn shut_down(&self) {
        todo!()
    }

    #[cfg(feature = "network-backpack")]
    pub(crate) fn start_network(&self, config: vertx_network::Config, api: &'static Api) {
        API.init(api).map_err(|_| ()).unwrap();
        self.tx.try_send(ToBackpack::StartNetwork(config)).unwrap();
    }
}

#[task]
async fn tx(
    mut tx: <crate::hal::Backpack as crate::hal::traits::Backpack>::Tx,
    messages: TxReceiver,
) -> ! {
    let mut buffer = [0; 256];

    loop {
        let message = messages.receive().await;
        let bytes = match postcard::to_slice_cobs(&message, &mut buffer) {
            Ok(bytes) => bytes,
            Err(err) => {
                log::error!("Failed to serialize message to backpack: {err:?}");
                continue;
            }
        };

        match tx.write_all(bytes).await {
            Ok(()) => {}
            Err(err) => {
                log::error!("Failed to send message to backpack: {err:?}");
            }
        }
    }
}

#[task]
async fn rx(
    mut rx: <crate::hal::Backpack as crate::hal::traits::Backpack>::Rx,
    tx: TxSender,
    #[cfg(feature = "backpack-boot-mode")] set_boot_mode_ack: &'static SetBootModeAck,
) {
    let mut api_buffer = Api::buffer();

    let mut ever_success = false;
    let mut buffer = [0; 32];
    let mut accumulator = CobsAccumulator::<256>::new();
    loop {
        let mut chunk = match rx.read(&mut buffer).await {
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
                        #[allow(unused)]
                        ToMain::Init { boot_mode } => {
                            #[cfg(feature = "backpack-boot-mode")]
                            BOOT_MODE.init(boot_mode).unwrap();
                            tx.send(ToBackpack::InitAck).await;
                        }

                        #[cfg(feature = "backpack-boot-mode")]
                        ToMain::SetBootModeAck => set_boot_mode_ack.signal(()),
                        #[cfg(not(feature = "backpack-boot-mode"))]
                        ToMain::SetBootModeAck => {
                            log::error!("Ignoring SetBootModeAck message from backpack")
                        }

                        #[cfg(feature = "network-backpack")]
                        ToMain::NetworkUp => log::info!("Network started"),
                        #[cfg(not(feature = "network-backpack"))]
                        ToMain::NetworkUp => {
                            log::error!("Ignoring NetworkUp message from backpack")
                        }

                        #[cfg(feature = "network-backpack")]
                        ToMain::ApiRequest(request) => {
                            if let Some(api) = API.try_get() {
                                if let Some(response) = api.handle(&request, &mut api_buffer).await
                                {
                                    tx.send(ToBackpack::ApiResponse(response.to_vec())).await;
                                }
                            } else {
                                log::error!("Got ApiRequest before network was initialized");
                            }
                        }
                        #[cfg(not(feature = "network-backpack"))]
                        ToMain::ApiRequest(_) => {
                            log::error!("Ignoring ApiRequest message from backpack")
                        }
                    }

                    remaining
                }
            }
        }
    }
}
