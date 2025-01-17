mod leds;

use display_interface::DisplayError;
use embassy_executor::Spawner;
use embassy_futures::select;
use embassy_rp::i2c::{self, I2c};
use embassy_rp::pio::{self, Pio};
use embassy_rp::uart::{self, BufferedUart};
use embassy_rp::watchdog::Watchdog;
use embassy_rp::{bind_interrupts, gpio, peripherals};
use embassy_time::Duration;
use embedded_alloc::TlsfHeap;
use static_cell::{ConstStaticCell, StaticCell};
use {defmt_rtt as _, embedded_graphics as eg, panic_probe as _};

use crate::ui::Input;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<peripherals::PIO0>;
    UART1_IRQ => uart::BufferedInterruptHandler<peripherals::UART1>;
    I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
});

declare_hal_types!();

#[global_allocator]
static ALLOCATOR: TlsfHeap = TlsfHeap::empty();

pub(super) fn init(_spawner: Spawner) -> super::Init {
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
        let pin = pins!(p, leds);
        leds::StatusDriver::<_, 0>::new(&mut common, sm0, pin)
    };

    let config_storage = ConfigStorage {};

    let ui = {
        let scl = pins!(p, display.scl);
        let sda = pins!(p, display.sda);
        let mut config = i2c::Config::default();
        config.frequency = 1_000_000;
        let display = super::display::new(I2c::new_async(p.I2C0, scl, sda, Irqs, config));

        Ui {
            display,
            up: gpio::Input::new(pins!(p, ui.up), gpio::Pull::Up),
            down: gpio::Input::new(pins!(p, ui.down), gpio::Pull::Up),
            right: gpio::Input::new(pins!(p, ui.right), gpio::Pull::Up),
            left: gpio::Input::new(pins!(p, ui.left), gpio::Pull::Up),
        }
    };

    let backpack = {
        static TX_BUFFER: ConstStaticCell<[u8; 32]> = ConstStaticCell::new([0; 32]);
        static RX_BUFFER: ConstStaticCell<[u8; 32]> = ConstStaticCell::new([0; 32]);

        let mut config = uart::Config::default();
        config.baudrate = vertx_backpack_ipc::BAUDRATE;
        let uart = BufferedUart::new(
            p.UART1,
            Irqs,
            pins!(p, backpack.tx),
            pins!(p, backpack.rx),
            TX_BUFFER.take(),
            RX_BUFFER.take(),
            config,
        );
        let (tx, rx) = uart.split();

        super::Backpack { tx, rx }
    };

    super::Init {
        reset,
        status_led,
        config_storage,
        ui,
        backpack,
    }
}

struct Reset {
    watchdog: Watchdog,
}

impl super::traits::Reset for Reset {
    fn shut_down(&mut self) -> ! {
        panic!("Emulating shut down")
    }

    fn reboot(&mut self) -> ! {
        self.watchdog.trigger_reset();
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

struct ConfigStorage {}

impl super::traits::ConfigStorage for ConfigStorage {
    fn load<T>(&self, _parse: impl FnOnce(&[u8]) -> Option<T>) -> Option<T> {
        // TODO
        None
    }

    fn save(&mut self, _config: &[u8]) {
        todo!()
    }
}

struct Ui {
    display: super::display::Driver<I2c<'static, peripherals::I2C0, i2c::Async>>,
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
