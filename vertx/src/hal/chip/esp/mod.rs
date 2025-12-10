#[expect(unused, reason = "preserve for future OTA updates")]
mod flash;
mod leds;
mod network;
mod ui;

use embassy_executor::Spawner;
use esp_hal::clock::CpuClock;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::gpio;
use esp_hal::i2c::master::{self as i2c, I2c};
use esp_hal::rmt::Rmt;
use esp_hal::rng::Rng;
use esp_hal::spi::master::{self as spi, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg;
use static_cell::ConstStaticCell;
use {defmt_rtt as _, esp_backtrace as _};

use crate::hal;

#[define_opaque(
    hal::Network,
    hal::Reset,
    hal::StatusLed,
    hal::Storage,
    hal::StorageFuture,
    hal::Ui
)]
pub(crate) fn init(spawner: Spawner) -> hal::Init {
    esp_alloc::heap_allocator!(size: 100 * 1024);

    let p = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    let rmt = Rmt::new(p.RMT, Rate::from_mhz(80)).unwrap().into_async();
    let rng = Rng::new(p.RNG);
    let timg0 = timg::TimerGroup::new(p.TIMG0);
    let timg1 = timg::TimerGroup::new(p.TIMG1);

    esp_hal_embassy::init(timg0.timer0);

    let status_led = leds::StatusLed::new(rmt.channel0, pins!(p, leds.status));

    let spi = {
        #[expect(clippy::manual_div_ceil)]
        let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = esp_hal::dma_buffers!(32000);
        let dma_rx = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
        let dma_tx = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

        Spi::new(p.SPI2, spi::Config::default())
            .unwrap()
            .with_sck(pins!(p, spi.sclk))
            .with_mosi(pins!(p, spi.mosi))
            .with_miso(pins!(p, spi.miso))
            .with_dma(p.DMA_CH0)
            .with_buffers(dma_rx, dma_tx)
            .into_async()
    };

    let storage = async {
        let sd_cs = gpio::Output::new(
            pins!(p, sd.cs),
            gpio::Level::High,
            gpio::OutputConfig::default(),
        );

        static BUFFERS: ConstStaticCell<vertx_filesystem::Buffers<aligned::A1>> =
            ConstStaticCell::new(vertx_filesystem::Buffers::new());
        let buffers = BUFFERS.take();

        crate::storage::sd_spi::new_exclusive_spi(buffers, spi, sd_cs, |spi, speed| {
            let config = spi::Config::default().with_frequency(Rate::from_hz(speed));
            spi.apply_config(&config).unwrap();
        })
        .await
    };

    let ui = {
        let config = i2c::Config::default().with_frequency(Rate::from_mhz(1));

        let i2c = I2c::new(p.I2C0, config)
            .unwrap()
            .with_sda(pins!(p, display.sda))
            .with_scl(pins!(p, display.scl))
            .into_async();

        let display = hal::display::new(i2c);

        let config = gpio::InputConfig::default().with_pull(gpio::Pull::Up);
        ui::Ui {
            display,
            up: gpio::Input::new(pins!(p, ui.up), config),
            down: gpio::Input::new(pins!(p, ui.down), config),
            right: gpio::Input::new(pins!(p, ui.right), config),
            left: gpio::Input::new(pins!(p, ui.left), config),
        }
    };

    hal::Init {
        reset: Reset,
        status_led,
        storage,
        ui,
        network: network::Network {
            spawner,
            rng,
            timer: timg1.timer0.into(),
            wifi: p.WIFI,
        },
    }
}

struct Reset;

impl hal::traits::Reset for Reset {
    fn shut_down(&mut self) -> ! {
        panic!("Emulating shut down")
    }

    fn reboot(&mut self) -> ! {
        esp_hal::system::software_reset()
    }
}
