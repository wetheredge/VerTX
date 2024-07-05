pub mod server;
pub mod wifi;

use embassy_executor::task;
use esp_hal::gpio;

pub use self::wifi::Config as WifiConfig;

#[task]
pub async fn toggle_button(mut button: gpio::AnyInput<'static>, reset: crate::reset::Manager) {
    button.wait_for_falling_edge().await;
    reset.toggle_configurator();
}
