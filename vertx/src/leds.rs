use core::iter;

use embassy_executor::task;
use embassy_futures::select;
use embassy_time::{Duration, Timer};
use esp_hal::rmt::Channel;
use esp_hal_smartled::SmartLedsAdapter;
use smart_leds::{colors, SmartLedsWrite, RGB8};

use crate::Mode;

pub const MAX_LEDS: usize = 1;
// 3 channels * 8 bits + 1 stop byte
pub const BUFFER_SIZE: usize = MAX_LEDS * 3 * 8 + 1;

#[derive(vertx_config::UpdateRef, vertx_config::Storage)]
pub struct Config {
    brightness: vertx_config::Reactive<u8, crate::mutex::SingleCore>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            brightness: 10.into(),
        }
    }
}

macro_rules! color_array {
    (static $name:ident = [ $(($r:expr, $g:expr, $b:expr)),* $(,)? ]) => {
        static $name: [RGB8; { $(1 + $r - $r +)* 0 }] = [$(RGB8::new($r, $g, $b)),*];
    };
}

color_array! {
    static RAINBOW = [
        (242, 138, 170), (244, 138, 161), (245, 139, 152), (246, 139, 142),
        (247, 141, 133), (246, 142, 124), (245, 144, 115), (243, 146, 106),
        (241, 149,  97), (238, 151,  89), (234, 154,  82), (230, 158,  75),
        (225, 161,  70), (219, 164,  66), (213, 168,  64), (206, 171,  63),
        (198, 175,  65), (190, 178,  68), (182, 181,  73), (173, 185,  80),
        (163, 187,  87), (153, 190,  95), (142, 193, 104), (131, 195, 113),
        (119, 197, 122), (107, 198, 132), ( 95, 200, 142), ( 81, 201, 151),
        ( 67, 201, 161), ( 52, 202, 170), ( 36, 201, 180), ( 15, 201, 189),
        (  0, 200, 197), (  0, 199, 206), (  0, 198, 214), (  8, 196, 222),
        ( 31, 195, 228), ( 47, 192, 235), ( 62, 190, 240), ( 76, 187, 245),
        ( 89, 185, 249), (101, 182, 253), (113, 179, 255), (125, 176, 255),
        (136, 173, 255), (146, 170, 255), (155, 167, 255), (164, 164, 255),
        (173, 161, 252), (181, 158, 249), (189, 155, 244), (197, 152, 239),
        (204, 150, 234), (210, 147, 227), (217, 145, 220), (222, 143, 213),
        (227, 142, 205), (232, 141, 197), (236, 139, 188), (239, 139, 179),
    ]
}

#[derive(Debug)]
enum Effect {
    Solid(RGB8),
    Blink {
        color1: RGB8,
        time1: Duration,
        color2: RGB8,
        time2: Duration,
        state: bool,
    },
    Rainbow {
        step: usize,
    },
}

impl Default for Effect {
    fn default() -> Self {
        Effect::Solid(colors::BLACK)
    }
}

impl From<Mode> for Effect {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Ok => Effect::Solid(colors::GREEN),
            Mode::Armed => Effect::Solid(colors::BLUE),
            Mode::PreConfigurator => Effect::blink(
                colors::MEDIUM_PURPLE,
                Duration::from_millis(500),
                colors::BLACK,
                Duration::from_millis(500),
            ),
            Mode::Configurator => Effect::Solid(colors::MEDIUM_PURPLE),
            Mode::Updating => Effect::rainbow(),
        }
    }
}

impl Effect {
    fn blink(color1: RGB8, time1: Duration, color2: RGB8, time2: Duration) -> Self {
        Self::Blink {
            color1,
            time1,
            color2,
            time2,
            state: false,
        }
    }

    fn rainbow() -> Self {
        Self::Rainbow {
            step: RAINBOW.len(),
        }
    }

    fn next_frame(&mut self) -> (RGB8, Option<Duration>) {
        match self {
            Self::Solid(color) => (*color, None),
            Self::Blink {
                color1,
                time1,
                color2,
                time2,
                ref mut state,
            } => {
                *state = !*state;
                if *state {
                    (*color1, Some(*time1))
                } else {
                    (*color2, Some(*time2))
                }
            }
            Self::Rainbow { ref mut step } => {
                *step = (*step + 1) % RAINBOW.len();
                (RAINBOW[*step], Some(Duration::from_hz(30)))
            }
        }
    }
}

#[task]
pub async fn run(
    config: &'static crate::Config,
    mut leds: SmartLedsAdapter<Channel<esp_hal::Blocking, 0>, { BUFFER_SIZE }>,
    mut mode: crate::mode::Subscriber<'static>,
) -> ! {
    log::info!("Starting leds()");
    let config = &config.leds;

    let mut effect = Effect::default();
    let mut brightness_subscriber = config.brightness.subscriber().unwrap();

    loop {
        let (color, duration) = effect.next_frame();
        let timer = duration.map(Timer::after);

        leds.write(smart_leds::brightness(
            smart_leds::gamma(iter::once(color)),
            *config.brightness.current().await,
        ))
        .unwrap();

        let new_mode = if let Some(timer) = timer {
            // Assume `timer` is a fraction of a second, so don't bother updating brightness
            // until the next frame
            match select::select(timer, mode.next()).await {
                select::Either::First(()) => continue,
                select::Either::Second(effect) => effect,
            }
        } else {
            match select::select(brightness_subscriber.wait(), mode.next()).await {
                select::Either::First(()) => continue,
                select::Either::Second(effect) => effect,
            }
        };

        effect = new_mode.into();
    }
}
