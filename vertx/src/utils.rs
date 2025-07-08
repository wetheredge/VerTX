use core::convert::Infallible;

use embassy_time::{Duration, Timer};

#[cfg_attr(any(test, feature = "simulator"), expect(unused))]
pub(crate) async fn debounced_falling_edge<P>(pin: &mut P, delay: Duration)
where
    P: embedded_hal_async::digital::Wait + embedded_hal::digital::InputPin<Error = Infallible>,
{
    loop {
        let Ok(()) = pin.wait_for_falling_edge().await;
        Timer::after(delay).await;
        let Ok(is_low) = pin.is_low();
        if is_low {
            return;
        }
    }
}
