mod leds;

use core::future::Future;

use embassy_executor::Spawner;
use embassy_rp::clocks::RoscRng;
use embassy_rp::peripherals::{PIO0, UART0};
use embassy_rp::pio::{self, Pio};
use embassy_rp::uart::{self, BufferedUart};
use embassy_rp::watchdog::Watchdog;
use embassy_rp::{bind_interrupts, gpio};
use panic_probe as _;
use static_cell::make_static;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    UART0_IRQ => uart::BufferedInterruptHandler<UART0>;
});

pub(crate) fn init(_spawner: Spawner) -> super::Init {
    let p = embassy_rp::init(Default::default());
    let rng = RoscRng;

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

    let mode_button = gpio::Input::new(pins!(p, mode), gpio::Pull::Up);

    let mut config = uart::Config::default();
    config.baudrate = vertx_backpack_ipc::BAUDRATE;
    let tx_buffer = make_static!([0; 32]);
    let rx_buffer = make_static!([0; 32]);
    let backpack = BufferedUart::new(
        p.UART0,
        Irqs,
        pins!(p, backpack.tx),
        pins!(p, backpack.rx),
        tx_buffer,
        rx_buffer,
        config,
    );

    super::Init {
        reset,
        rng,
        led_driver,
        config_storage,
        mode_button,
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
    fn load<T>(&self, parse: impl FnOnce(&[u8]) -> T) -> Option<T> {
        // TODO
        None
    }

    fn save(&mut self, data: &[u32]) {
        todo!()
    }
}

impl super::traits::ModeButton for gpio::Input<'_> {
    fn wait_for_pressed(&mut self) -> impl Future<Output = ()> {
        self.wait_for_falling_edge()
    }
}

impl<'d, T: uart::Instance> super::traits::Backpack for BufferedUart<'d, T> {
    type Rx = uart::BufferedUartRx<'d, T>;
    type Tx = uart::BufferedUartTx<'d, T>;

    fn split(self) -> (Self::Tx, Self::Rx) {
        self.split()
    }
}
