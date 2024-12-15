mod flash;

use alloc::vec;
use alloc::vec::Vec;

use display_interface::DisplayError;
use embassy_executor::Spawner;
use embassy_futures::select;
use embassy_time::Duration;
use esp_hal::clock::CpuClock;
use esp_hal::gpio;
use esp_hal::i2c::I2c;
use esp_hal::prelude::*;
use esp_hal::rmt::Rmt;
use esp_hal::rng::Rng;
use esp_hal::timer::timg;
use esp_hal_smartled::SmartLedsAdapter;
use portable_atomic::{AtomicU8, Ordering};
use {embedded_graphics as eg, esp_backtrace as _, esp_println as _};

use self::flash::Partition;
use crate::ui::Input;
use crate::BootMode;

#[ram(rtc_fast, persistent)]
static BOOT_MODE: AtomicU8 = AtomicU8::new(0);

declare_hal_types!();

pub(super) fn init(spawner: Spawner) -> super::Init {
    // TODO: increase size?
    esp_alloc::heap_allocator!(32 * 1024);

    let peripherals = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    let io = gpio::Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let rmt = Rmt::new(peripherals.RMT, 80u32.MHz()).unwrap();
    let rng = Rng::new(peripherals.RNG);
    let timg0 = timg::TimerGroup::new(peripherals.TIMG0);
    let timg1 = timg::TimerGroup::new(peripherals.TIMG1);

    esp_hal_embassy::init(timg0.timer0);

    let status_led = SmartLedsAdapter::new(
        rmt.channel0,
        pins!(io, leds),
        [0; 3 * 8 + 1], // 3 channels * 8 bits + 1 stop byte
    );

    flash::unlock().unwrap();
    let mut partitions = flash::read_partition_table()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let config_storage = ConfigStorage::new(&mut partitions);

    let ui = {
        let sda = pins!(io, display.sda);
        let scl = pins!(io, display.scl);
        let display = super::display::new(I2c::new_async(peripherals.I2C0, sda, scl, 1.MHz()));

        Ui {
            display,
            up: gpio::Input::new(pins!(io, ui.up), gpio::Pull::Up),
            down: gpio::Input::new(pins!(io, ui.down), gpio::Pull::Up),
            right: gpio::Input::new(pins!(io, ui.right), gpio::Pull::Up),
            left: gpio::Input::new(pins!(io, ui.left), gpio::Pull::Up),
        }
    };

    let network_hal = vertx_network_esp::Hal::new(
        spawner,
        rng,
        timg1.timer0.into(),
        peripherals.RADIO_CLK,
        peripherals.WIFI,
    );

    super::Init {
        reset: Reset,
        boot_mode: BootMode::from(BOOT_MODE.load(Ordering::Relaxed)),
        status_led,
        config_storage,
        ui,
        network: Network::new(rng, network_hal),
    }
}

pub(super) fn set_boot_mode(mode: u8) {
    BOOT_MODE.store(mode, Ordering::Relaxed);
}

struct Reset;

impl super::traits::Reset for Reset {
    fn shut_down(&mut self) -> ! {
        panic!("Emulating shut down")
    }

    fn reboot(&mut self) -> ! {
        esp_hal::reset::software_reset();
        unreachable!()
    }
}

struct ConfigStorage {
    partition: Partition,
}

impl ConfigStorage {
    fn new(partitions: &mut Vec<Partition>) -> Self {
        let partition = partitions.iter().position(Partition::is_config).unwrap();
        Self {
            partition: partitions.swap_remove(partition),
        }
    }
}

impl super::traits::ConfigStorage for ConfigStorage {
    fn load<T>(&self, parse: impl FnOnce(&[u8]) -> Option<T>) -> Option<T> {
        let mut length = [0; 1];
        self.partition.read_into(0, &mut length).unwrap();
        let [length] = length;
        let length = if length == u32::MAX { 0 } else { length };
        let length = length as usize;

        if length > 0 {
            let mut config = vec![0; length];
            self.partition.read_into(1, &mut config).unwrap();

            let bytes: &[u8] = &bytemuck::cast_slice(&config)[0..length];
            parse(bytes)
        } else {
            None
        }
    }

    fn save(&mut self, config: &[u8]) {
        let mut buffer = [0; crate::config::BYTE_LENGTH.div_ceil(4)];
        // Ensure word alignment
        bytemuck::cast_slice_mut(&mut buffer).copy_from_slice(config);

        let sectors = (config.len() as u32).div_ceil(flash::SECTOR_BYTES);
        for i in 0..sectors {
            self.partition.erase_sector(i).unwrap();
        }

        let len_words = config.len().div_ceil(4);
        self.partition.write(0, &[len_words as u32]).unwrap();
        self.partition.write(1, &buffer[0..len_words]).unwrap();
    }
}

struct Network {
    rng: Rng,
    hal: vertx_network_esp::Hal,
}

impl Network {
    fn new(rng: Rng, hal: vertx_network_esp::Hal) -> Self {
        Self { rng, hal }
    }
}

impl super::traits::Network for Network {
    type Hal = vertx_network_esp::Hal;

    fn seed(&mut self) -> u64 {
        use rand::RngCore;
        self.rng.next_u64()
    }

    fn hal(self) -> Self::Hal {
        self.hal
    }
}

struct Ui {
    display: super::display::Driver<I2c<'static, esp_hal::peripherals::I2C0, esp_hal::Async>>,
    up: gpio::Input<'static>,
    down: gpio::Input<'static>,
    right: gpio::Input<'static>,
    left: gpio::Input<'static>,
}

impl eg::geometry::OriginDimensions for Ui {
    fn size(&self) -> eg::geometry::Size {
        super::display::SIZE
    }
}

impl eg::draw_target::DrawTarget for Ui {
    type Color = eg::pixelcolor::BinaryColor;
    type Error = DisplayError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = eg::Pixel<Self::Color>>,
    {
        self.display.draw_iter(pixels)
    }
}

impl super::traits::Ui for Ui {
    async fn init(&mut self) -> Result<(), Self::Error> {
        super::display::init(&mut self.display).await
    }

    async fn get_input(&mut self) -> Input {
        async fn debounced(pin: &mut gpio::Input<'static>, input: Input) -> Input {
            crate::utils::debounced_falling_edge(pin, Duration::from_millis(20)).await;
            input
        }

        let up = debounced(&mut self.up, Input::Up);
        let down = debounced(&mut self.down, Input::Down);
        let right = debounced(&mut self.right, Input::Forward);
        let left = debounced(&mut self.left, Input::Back);

        select::select_array([up, down, left, right]).await.0
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.display.flush().await
    }
}
