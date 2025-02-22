use embassy_executor::{task, Spawner};
use embassy_futures::select;
use embassy_sync::channel;
use embassy_sync::once_lock::OnceLock;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};
use embedded_io_async::{Read as _, Write as _};
use postcard::accumulator::{CobsAccumulator, FeedResult};
use static_cell::ConstStaticCell;
use vertx_backpack_ipc::{ToBackpack, ToMain, INIT};
use vertx_network::Api as _;

use crate::api::Api;

type TxChannel = channel::Channel<crate::mutex::SingleCore, ToBackpack, 10>;
type TxSender = channel::Sender<'static, crate::mutex::SingleCore, ToBackpack, 10>;
type TxReceiver = channel::Receiver<'static, crate::mutex::SingleCore, ToBackpack, 10>;

type NetworkUp = Signal<crate::mutex::SingleCore, ()>;
type PowerAck = Signal<crate::mutex::SingleCore, ()>;

#[cfg(feature = "network-backpack")]
static API: OnceLock<&'static Api> = OnceLock::new();

#[derive(Clone)]
pub(crate) struct Backpack {
    tx: TxSender,
    network_up: &'static NetworkUp,
    power_ack: &'static PowerAck,
}

impl Backpack {
    pub(crate) fn new(
        init: &'static crate::InitCounter,
        spawner: Spawner,
        backpack: crate::hal::Backpack,
    ) -> Self {
        let init = init.start(loog::intern!("backpack"));

        static CHANNEL: ConstStaticCell<TxChannel> = ConstStaticCell::new(TxChannel::new());
        static NETWORK_UP: ConstStaticCell<NetworkUp> = ConstStaticCell::new(Signal::new());
        static POWER_ACK: ConstStaticCell<PowerAck> = ConstStaticCell::new(Signal::new());

        let channel = CHANNEL.take();
        let network_up = NETWORK_UP.take();
        let power_ack = POWER_ACK.take();

        spawner.must_spawn(init_and_tx(
            init, spawner, backpack, channel, network_up, power_ack,
        ));

        Self {
            tx: channel.sender(),
            network_up,
            power_ack,
        }
    }

    pub(crate) async fn shut_down(&self) {
        self.tx.send(ToBackpack::ShutDown).await;
        self.power_ack.wait().await;
    }

    pub(crate) async fn reboot(&self) {
        self.tx.send(ToBackpack::Reboot).await;
        self.power_ack.wait().await;
    }

    #[cfg(feature = "network-backpack")]
    pub(crate) async fn start_network(&self, config: vertx_network::Config, api: &'static Api) {
        API.init(api).map_err(|_| ()).unwrap();
        self.tx.send(ToBackpack::StartNetwork(config)).await;
        self.network_up.wait().await;
    }
}

#[task]
async fn init_and_tx(
    init_tracker: crate::init_counter::Tracker,
    spawner: Spawner,
    mut backpack: crate::hal::Backpack,
    messages: &'static TxChannel,
    network_up: &'static NetworkUp,
    power_ack: &'static PowerAck,
) -> ! {
    init(&mut backpack).await;
    init_tracker.finish();

    let crate::hal::Backpack { tx, rx } = backpack;
    spawner.must_spawn(rx_handler(rx, messages.sender(), network_up, power_ack));
    tx_handler(tx, messages.receiver()).await
}

async fn init(backpack: &mut crate::hal::Backpack) {
    let mut ticker = Ticker::every(Duration::from_millis(10));
    loop {
        backpack.tx.write_all(&INIT).await.unwrap();

        let mut init = [0; INIT.len()];
        match select::select(ticker.next(), backpack.rx.read(&mut init)).await {
            select::Either::First(()) => {}
            select::Either::Second(len) => {
                let Ok(mut len) = len else {
                    continue;
                };

                if init[0] == INIT[0] {
                    while len < init.len() {
                        match backpack.rx.read(&mut init[len..]).await {
                            Ok(new_len) => len += new_len,
                            Err(_) => continue,
                        }
                    }

                    // Successfully rxed repeated INIT message
                    if init == INIT {
                        break;
                    }
                }
            }
        }
    }
}

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
    mut rx: crate::hal::BackpackRx,
    tx: TxSender,
    network_up: &'static NetworkUp,
    power_ack: &'static PowerAck,
) -> ! {
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
                    match data {
                        #[cfg(feature = "network-backpack")]
                        ToMain::NetworkUp => network_up.signal(()),
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

                        ToMain::PowerAck => power_ack.signal(()),
                    }

                    remaining
                }
            }
        }
    }
}
