mod leds;

use embassy_executor::Spawner;
use embassy_futures::select;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::i2c::{self, I2c};
use embassy_stm32::mode::Async;
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::{bind_interrupts, gpio, peripherals, time};
use embassy_time::Duration;
use embedded_alloc::TlsfHeap;
use static_cell::StaticCell;
use {defmt_rtt as _, embedded_graphics as eg, panic_probe as _};

use crate::hal;
use crate::storage::sd;
use crate::ui::Input;

bind_interrupts!(struct Irqs {
    #[cfg(peripheral = "I2C1")]
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    #[cfg(peripheral = "I2C1")]
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;

    #[cfg(peripheral = "I2C2")]
    I2C2_EV => i2c::EventInterruptHandler<peripherals::I2C2>;
    #[cfg(peripheral = "I2C2")]
    I2C2_ER => i2c::ErrorInterruptHandler<peripherals::I2C2>;

    #[cfg(peripheral = "I2C3")]
    I2C3_EV => i2c::EventInterruptHandler<peripherals::I2C3>;
    #[cfg(peripheral = "I2C3")]
    I2C3_ER => i2c::ErrorInterruptHandler<peripherals::I2C3>;
});

declare_hal_types!();

#[global_allocator]
static ALLOCATOR: TlsfHeap = TlsfHeap::empty();

pub(crate) fn init(_spawner: Spawner) -> hal::Init {
    static INIT_HEAP: StaticCell<()> = StaticCell::new();
    INIT_HEAP.init_with(|| {
        use core::mem::MaybeUninit;

        const HEAP_SIZE: usize = 32 * 1024;

        // SAFETY: this pointer is guaranteed to be unique:
        // - `StaticCell::init_with` guarantees this can only run once and
        // - No other code can access `HEAP`
        #[allow(static_mut_refs)]
        let start = unsafe {
            static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();
            HEAP.as_mut_ptr()
        };

        // SAFETY:
        // - `StaticCell` guarantees this will only be called once
        // - `HEAP_SIZE` is > 0
        unsafe { ALLOCATOR.init(start as usize, HEAP_SIZE) };
    });

    let config = embassy_stm32::Config::default();
    let p = embassy_stm32::init(config);

    let status_led = leds::StatusDriver::new(
        target!(p, leds.timer),
        target!(p, leds.dma),
        target!(p, leds.status),
    );

    let spi = {
        let mut config = spi::Config::default();
        config.rise_fall_speed = gpio::Speed::VeryHigh; // FIXME: needed?
        Spi::new(
            target!(p, spi),
            target!(p, spi.sclk),
            target!(p, spi.mosi),
            target!(p, spi.miso),
            target!(p, spi.dma.tx),
            target!(p, spi.dma.rx),
            config,
        )
    };

    let storage = async {
        // Using impl Trait hits the `item does not constrain but has it in its
        // signature` error with the static below. Something neater would be nice, but
        // this works :/
        type SpiDevice = embedded_hal_bus::spi::ExclusiveDevice<
            Spi<'static, Async>,
            gpio::Output<'static>,
            embassy_time::Delay,
        >;

        static STORAGE: StaticCell<sd::Storage<SpiDevice>> = StaticCell::new();
        let sd_cs = gpio::Output::new(target!(p, sd.cs), gpio::Level::High, gpio::Speed::VeryHigh); // FIXME: speed?
        let storage = sd::Storage::new_exclusive_spi(spi, sd_cs, |spi, speed| {
            let mut config = spi.get_current_config();
            config.frequency = time::hz(speed.to_Hz());
            spi.set_config(&config).unwrap();
        })
        .await;
        let storage: &'static _ = STORAGE.init(storage);
        storage
    };

    let ui = {
        let display = hal::display::new(I2c::new(
            target!(p, display.i2c),
            target!(p, display.scl),
            target!(p, display.sda),
            Irqs,
            target!(p, display.dma.tx),
            target!(p, display.dma.rx),
            time::mhz(1),
            i2c::Config::default(),
        ));

        Ui {
            display,
            up: target!(p, ExtiInput(ui.up, gpio::Pull::Up)),
            down: target!(p, ExtiInput(ui.down, gpio::Pull::Up)),
            right: target!(p, ExtiInput(ui.right, gpio::Pull::Up)),
            left: target!(p, ExtiInput(ui.left, gpio::Pull::Up)),
        }
    };

    hal::Init {
        reset: Reset,
        status_led,
        storage,
        ui,
    }
}

struct Reset;

impl hal::traits::Reset for Reset {
    fn shut_down(&mut self) -> ! {
        loog::panic!("Emulating shut down")
    }

    fn reboot(&mut self) -> ! {
        loog::todo!("reboot")
    }
}

struct Ui {
    display: hal::display::Driver<I2c<'static, Async>>,
    up: ExtiInput<'static>,
    down: ExtiInput<'static>,
    right: ExtiInput<'static>,
    left: ExtiInput<'static>,
}

impl eg::geometry::OriginDimensions for Ui {
    fn size(&self) -> eg::geometry::Size {
        hal::display::SIZE
    }
}

impl eg::draw_target::DrawTarget for Ui {
    type Color = eg::pixelcolor::BinaryColor;
    type Error = display_interface::DisplayError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = eg::Pixel<Self::Color>>,
    {
        self.display.draw_iter(pixels)
    }
}

impl hal::traits::Ui for Ui {
    async fn init(&mut self) -> Result<(), Self::Error> {
        hal::display::init(&mut self.display).await
    }

    async fn get_input(&mut self) -> crate::ui::Input {
        let debounced = async |pin, input| {
            crate::utils::debounced_falling_edge(pin, Duration::from_millis(20)).await;
            input
        };

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
