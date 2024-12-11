mod leds;

use core::convert::Infallible;

use embassy_executor::Spawner;
use embassy_rp::peripherals::{PIO0, UART1};
use embassy_rp::pio::{self, Pio};
use embassy_rp::uart::{self, BufferedUart};
use embassy_rp::watchdog::Watchdog;
use embassy_rp::{bind_interrupts, gpio};
use embedded_alloc::TlsfHeap;
use static_cell::{ConstStaticCell, StaticCell};
use {defmt_rtt as _, embedded_graphics as eg, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    UART1_IRQ => uart::BufferedInterruptHandler<UART1>;
});

declare_hal_types!();

#[global_allocator]
static ALLOCATOR: TlsfHeap = TlsfHeap::empty();

pub(super) fn init(_spawner: Spawner) -> super::Init {
    static INIT_HEAP: StaticCell<()> = StaticCell::new();
    INIT_HEAP.init_with(|| {
        use core::mem::MaybeUninit;

        const HEAP_SIZE: usize = 32 * 1024;
        static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

        // SAFETY:
        // - `StaticCell` guarantees this will only be called once
        // - `HEAP_SIZE` is > 0
        unsafe { ALLOCATOR.init(HEAP.as_mut_ptr() as usize, HEAP_SIZE) };
    });

    let p = embassy_rp::init(Default::default());

    let reset = Reset {
        watchdog: Watchdog::new(p.WATCHDOG),
    };

    let led_driver = {
        let Pio {
            mut common, sm0, ..
        } = Pio::new(p.PIO0, Irqs);
        let pin = pins!(p, leds);
        leds::Driver::<_, 0, 1>::new(&mut common, sm0, p.DMA_CH0, pin)
    };

    let config_storage = ConfigStorage {};

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
        led_driver,
        config_storage,
        ui: Ui,
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
        #[allow(clippy::empty_loop)]
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

struct Ui;

impl eg::geometry::OriginDimensions for Ui {
    fn size(&self) -> eg::geometry::Size {
        eg::geometry::Size {
            width: 128,
            height: 64,
        }
    }
}

impl eg::draw_target::DrawTarget for Ui {
    type Color = eg::pixelcolor::BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = eg::Pixel<Self::Color>>,
    {
        todo!()
    }
}

impl super::traits::Ui for Ui {
    async fn init(&mut self) -> Result<(), Self::Error> {
        todo!()
    }

    async fn get_input(&mut self) -> crate::ui::Input {
        todo!()
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        todo!()
    }
}
