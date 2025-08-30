mod leds;
mod ui;

use embassy_executor::Spawner;
use embassy_rp::i2c::{self, I2c};
use embassy_rp::pio::{self, Pio};
use embassy_rp::spi::{self, Spi};
use embassy_rp::watchdog::Watchdog;
use embassy_rp::{bind_interrupts, gpio, peripherals, usb};
use embedded_alloc::TlsfHeap;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use crate::hal;
use crate::storage::sd;

bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<peripherals::I2C0>;
    PIO0_IRQ_0 => pio::InterruptHandler<peripherals::PIO0>;
    USBCTRL_IRQ => usb::InterruptHandler<peripherals::USB>;
});

#[global_allocator]
static ALLOCATOR: TlsfHeap = TlsfHeap::empty();

#[define_opaque(
    hal::GetNetworkSeed,
    hal::Reset,
    hal::StatusLed,
    hal::StorageFuture,
    hal::Ui,
    hal::Usb
)]
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

        ui::Ui {
            display,
            up: gpio::Input::new(pins!(p, ui.up), gpio::Pull::Up),
            down: gpio::Input::new(pins!(p, ui.down), gpio::Pull::Up),
            right: gpio::Input::new(pins!(p, ui.right), gpio::Pull::Up),
            left: gpio::Input::new(pins!(p, ui.left), gpio::Pull::Up),
        }
    };

    let usb = usb::Driver::new(p.USB, Irqs);

    hal::Init {
        reset,
        status_led,
        storage,
        ui,
        usb,
        get_network_seed: || embassy_rp::clocks::RoscRng.next_u64(),
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
