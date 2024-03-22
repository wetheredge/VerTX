#![allow(unused)]

use embassy_executor::{task, Spawner};
use embassy_futures::select::{select, Either};
use embassy_sync::channel::{self, Channel};
use embassy_sync::signal::Signal;
use embassy_sync::zerocopy_channel;
use embassy_time::{Duration, Timer};
use embedded_io_async::Read;
use esp_hal::clock::Clocks;
use esp_hal::gpio::{self, AnyPin, GpioPin};
use esp_hal::peripherals::UART1;
use esp_hal::uart::{self, TxRxPins, UartPins};
use esp_hal::{Uart, UartRx, UartTx};
use vertx_crsf::{Address, DecodeError, EncodeError, Packet, RcChannelsPacked};

pub type TxChannel = Channel<crate::mutex::MultiCore, Packet, 10>;
pub type TxChannelSender = channel::Sender<'static, crate::mutex::MultiCore, Packet, 10>;
type TxChannelReceiver = channel::Receiver<'static, crate::mutex::MultiCore, Packet, 10>;
pub type RxChannel = Channel<crate::mutex::MultiCore, Packet, 10>;
type RxChannelSender = channel::Sender<'static, crate::mutex::MultiCore, Packet, 10>;
pub type RxChannelReceiver = channel::Receiver<'static, crate::mutex::MultiCore, Packet, 10>;
pub type RcSignal = Signal<crate::mutex::MultiCore, [u16; 16]>;

pub fn run(
    spawner: &Spawner,
    uart: UART1,
    tx_pin: GpioPin<gpio::Unknown, 17>,
    rx_pin: GpioPin<gpio::Unknown, 18>,
    rc: &'static RcSignal,
    tx: TxChannelReceiver,
    rx: RxChannelSender,
    clocks: &Clocks<'_>,
) {
    let config = uart::config::Config {
        baudrate: 115_200,
        ..Default::default()
    };
    let pins = TxRxPins::new_tx_rx(tx_pin, rx_pin);
    let mut uart = Uart::new_with_config(uart, config, Some(pins), clocks);
    let (uart_tx, uart_rx) = uart.split();
    // spawner.must_spawn(read(uart_rx, rx));
    spawner.must_spawn(write(uart_tx, rc, tx));
}

#[task]
async fn read(mut rx: UartRx<'static, UART1>, channel: RxChannelSender) -> ! {
    let mut buffer = [0; 1024];
    loop {
        let len = rx.read(&mut buffer).await.unwrap();

        // let count = buffer[..len].iter().filter(|&&b| b == 0xc8).count();
        // log::warn!("Found {count} possible sync bytes");

        let mut start = 0;
        while start < len {
            match Packet::decode(&buffer[start..len]) {
                Ok((packet, read)) => {
                    start += len;
                    esp_println::dbg!(&packet);
                    channel.send(packet).await;
                    continue;
                }
                Err(DecodeError::InvalidSyncByte(_)) => {}
                Err(err) => log::warn!("{err:?}"),
            }

            start += 1;
        }
    }
}

#[task]
async fn write(mut tx: UartTx<'static, UART1>, rc: &'static RcSignal, channel: TxChannelReceiver) {
    let mut buffer = [0; 1024];

    loop {
        let packet = match select(rc.wait(), channel.receive()).await {
            Either::First(rc) => Packet::RcChannelsPacked(RcChannelsPacked::new(rc)),
            Either::Second(packet) => packet,
        };

        let len = packet.encode(&mut buffer).unwrap();
        let _ = tx.write_bytes(&buffer[..len]);
    }
}
