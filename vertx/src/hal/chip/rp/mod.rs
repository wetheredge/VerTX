mod leds;

use display_interface::DisplayError;
use embassy_executor::Spawner;
use embassy_futures::select;
use embassy_rp::i2c::{self, I2c};
use embassy_rp::pio::{self, Pio};
use embassy_rp::spi::{self, Spi};
use embassy_rp::watchdog::Watchdog;
use embassy_rp::{bind_interrupts, gpio, peripherals};
use embassy_time::Duration;
use embedded_alloc::TlsfHeap;
use static_cell::StaticCell;
use {defmt_rtt as _, embedded_graphics as eg, panic_probe as _};

use crate::hal;
use crate::storage::sd;
use crate::ui::Input;

bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
    PIO0_IRQ_0 => pio::InterruptHandler<peripherals::PIO0>;
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

    let p = embassy_rp::init(Default::default());

    let reset = Reset {
        watchdog: Watchdog::new(p.WATCHDOG),
    };

    let status_led = {
        let Pio {
            mut common, sm0, ..
        } = Pio::new(p.PIO0, Irqs);
        let pin = pins!(p, leds.status);
        leds::StatusDriver::<_, 0>::new(&mut common, sm0, pin)
    };

    let spi = Spi::new(
        p.SPI1,
        pins!(p, spi.sclk),
        pins!(p, spi.mosi),
        pins!(p, spi.miso),
        p.DMA_CH0,
        p.DMA_CH1,
        spi::Config::default(),
    );

    let storage = async {
        // Using impl Trait hits the `item does not constrain but has it in its
        // signature` error with the static below. Something neater would be nice, but
        // this works :/
        type SpiDevice = embedded_hal_bus::spi::ExclusiveDevice<
            Spi<'static, peripherals::SPI1, spi::Async>,
            gpio::Output<'static>,
            embassy_time::Delay,
        >;

        static STORAGE: StaticCell<sd::Storage<SpiDevice>> = StaticCell::new();
        let sd_cs = gpio::Output::new(pins!(p, sd.cs), gpio::Level::High);
        let storage = sd::Storage::new_exclusive_spi(spi, sd_cs, Spi::set_frequency).await;
        let storage: &'static _ = STORAGE.init(storage);
        storage
    };

    let ui = {
        let scl = pins!(p, display.scl);
        let sda = pins!(p, display.sda);
        let mut config = i2c::Config::default();
        config.frequency = 1_000_000;
        let display = hal::display::new(I2c::new_async(p.I2C0, scl, sda, Irqs, config));

        Ui {
            display,
            up: gpio::Input::new(pins!(p, ui.up), gpio::Pull::Up),
            down: gpio::Input::new(pins!(p, ui.down), gpio::Pull::Up),
            right: gpio::Input::new(pins!(p, ui.right), gpio::Pull::Up),
            left: gpio::Input::new(pins!(p, ui.left), gpio::Pull::Up),
        }
    };

    hal::Init {
        reset,
        status_led,
        storage,
        ui,
    }
}

struct Reset {
    watchdog: Watchdog,
}

impl hal::traits::Reset for Reset {
    fn shut_down(&mut self) -> ! {
        panic!("Emulating shut down")
    }

    fn reboot(&mut self) -> ! {
        self.watchdog.trigger_reset();
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

struct Ui {
    display: hal::display::Driver<I2c<'static, peripherals::I2C0, i2c::Async>>,
    up: gpio::Input<'static>,
    down: gpio::Input<'static>,
    right: gpio::Input<'static>,
    left: gpio::Input<'static>,
}

impl eg::geometry::OriginDimensions for Ui {
    fn size(&self) -> eg::geometry::Size {
        hal::display::SIZE
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

impl hal::traits::Ui for Ui {
    async fn init(&mut self) -> Result<(), Self::Error> {
        hal::display::init(&mut self.display).await
    }

    async fn get_input(&mut self) -> crate::ui::Input {
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
