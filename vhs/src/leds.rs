use core::iter;

use embassy_executor::task;
use embassy_futures::select;
use embassy_time::{Duration, Timer};
use esp_hal_smartled::SmartLedsAdapter;
use hal::rmt::Channel;

use smart_leds::{
    colors,
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite, RGB8,
};

use crate::Status;

pub const MAX_LEDS: usize = 1;
// 3 channels * 8 bits + 1 stop byte
pub const BUFFER_SIZE: usize = MAX_LEDS * 3 * 8 + 1;

const MAX_BRIGHTNESS: u8 = 10;

static RAINBOW: [RGB8; 90] = [
    RGB8::new(220, 127, 155),
    RGB8::new(222, 126, 150),
    RGB8::new(223, 127, 144),
    RGB8::new(224, 127, 138),
    RGB8::new(224, 127, 133),
    RGB8::new(225, 128, 127),
    RGB8::new(225, 129, 121),
    RGB8::new(225, 129, 116),
    RGB8::new(224, 130, 110),
    RGB8::new(223, 132, 105),
    RGB8::new(222, 133, 100),
    RGB8::new(221, 134, 94),
    RGB8::new(220, 136, 89),
    RGB8::new(218, 138, 85),
    RGB8::new(216, 139, 80),
    RGB8::new(213, 141, 76),
    RGB8::new(211, 143, 72),
    RGB8::new(208, 145, 68),
    RGB8::new(205, 147, 65),
    RGB8::new(201, 149, 62),
    RGB8::new(198, 151, 61),
    RGB8::new(194, 153, 59),
    RGB8::new(190, 155, 59),
    RGB8::new(186, 158, 59),
    RGB8::new(181, 160, 61),
    RGB8::new(176, 162, 62),
    RGB8::new(171, 164, 65),
    RGB8::new(166, 166, 68),
    RGB8::new(160, 167, 72),
    RGB8::new(155, 169, 76),
    RGB8::new(149, 171, 80),
    RGB8::new(143, 173, 85),
    RGB8::new(136, 174, 90),
    RGB8::new(130, 176, 95),
    RGB8::new(123, 177, 101),
    RGB8::new(116, 178, 106),
    RGB8::new(109, 179, 112),
    RGB8::new(102, 180, 118),
    RGB8::new(95, 181, 124),
    RGB8::new(87, 182, 129),
    RGB8::new(79, 183, 135),
    RGB8::new(71, 183, 141),
    RGB8::new(63, 184, 147),
    RGB8::new(55, 184, 153),
    RGB8::new(46, 184, 158),
    RGB8::new(37, 184, 164),
    RGB8::new(27, 184, 170),
    RGB8::new(17, 183, 175),
    RGB8::new(6, 183, 180),
    RGB8::new(0, 182, 185),
    RGB8::new(0, 181, 190),
    RGB8::new(3, 181, 195),
    RGB8::new(13, 180, 200),
    RGB8::new(23, 179, 204),
    RGB8::new(33, 177, 208),
    RGB8::new(42, 176, 212),
    RGB8::new(50, 175, 216),
    RGB8::new(59, 173, 219),
    RGB8::new(67, 172, 222),
    RGB8::new(74, 170, 225),
    RGB8::new(82, 169, 227),
    RGB8::new(89, 167, 229),
    RGB8::new(96, 165, 231),
    RGB8::new(103, 163, 233),
    RGB8::new(110, 161, 234),
    RGB8::new(117, 160, 234),
    RGB8::new(123, 158, 235),
    RGB8::new(129, 156, 235),
    RGB8::new(135, 154, 234),
    RGB8::new(141, 152, 234),
    RGB8::new(147, 150, 233),
    RGB8::new(152, 148, 231),
    RGB8::new(158, 147, 230),
    RGB8::new(163, 145, 228),
    RGB8::new(168, 143, 225),
    RGB8::new(173, 141, 223),
    RGB8::new(177, 140, 220),
    RGB8::new(182, 138, 216),
    RGB8::new(186, 137, 213),
    RGB8::new(190, 135, 209),
    RGB8::new(194, 134, 205),
    RGB8::new(197, 133, 201),
    RGB8::new(201, 132, 196),
    RGB8::new(204, 131, 192),
    RGB8::new(207, 130, 187),
    RGB8::new(210, 129, 182),
    RGB8::new(212, 128, 177),
    RGB8::new(215, 128, 172),
    RGB8::new(217, 127, 166),
    RGB8::new(219, 127, 161),
];

#[derive(Debug)]
pub enum LedEffect {
    Solid(RGB8),
    Blink {
        color1: RGB8,
        time1: Duration,
        color2: RGB8,
        time2: Duration,
        state: bool,
    },
    Rainbow {
        hue: u8,
        // step: usize,
    },
}

impl Default for LedEffect {
    fn default() -> Self {
        LedEffect::Solid(colors::BLACK)
    }
}

impl From<Status> for LedEffect {
    fn from(status: Status) -> Self {
        match status {
            Status::Ok => LedEffect::Solid(colors::GREEN),
            Status::Armed => LedEffect::Solid(colors::BLUE),
            Status::PreWiFi => LedEffect::blink(
                colors::MEDIUM_PURPLE,
                Duration::from_millis(500),
                colors::BLACK,
                Duration::from_millis(500),
            ),
            Status::WiFi => LedEffect::Solid(colors::MEDIUM_PURPLE),
            Status::Updating => LedEffect::rainbow(),
        }
    }
}

impl LedEffect {
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
        Self::Rainbow { hue: 255 }
        // Self::Rainbow {
        //     step: RAINBOW.len(),
        // }
    }

    fn next(&mut self) -> (RGB8, Option<Duration>) {
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

            Self::Rainbow { ref mut hue } => {
                *hue = hue.wrapping_add(1);

                (
                    hsv2rgb(Hsv {
                        hue: *hue,
                        sat: 255,
                        val: 255,
                    }),
                    Some(Duration::from_secs(3) / 256),
                )
            } // Self::Rainbow { ref mut step } => {
              //     *step = (*step + 1) % RAINBOW.len();
              //     (RAINBOW[*step], Some(Duration::from_hz(30)))
              // }
        }
    }
}

#[task]
pub async fn run(
    mut leds: SmartLedsAdapter<Channel<0>, { BUFFER_SIZE }>,
    mut status: crate::status::Subscriber<'static>,
) -> ! {
    log::info!("Starting leds()");

    let mut effect = LedEffect::default();

    loop {
        let (color, new_timer) = effect.next();

        leds.write(smart_leds::brightness(
            smart_leds::gamma(iter::once(color)),
            MAX_BRIGHTNESS,
        ))
        .unwrap();

        let new_status = if let Some(new_next) = new_timer {
            match select::select(Timer::after(new_next), status.next()).await {
                select::Either::First(_) => continue,
                select::Either::Second(effect) => effect,
            }
        } else {
            status.next().await
        };

        effect = new_status.into();
    }
}
