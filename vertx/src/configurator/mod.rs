pub mod server;
pub mod wifi;

use embassy_executor::task;

pub use self::wifi::Config as WifiConfig;

#[task]
pub async fn button(pressed: crate::hal::ConfiguratorButton, reset: crate::reset::Manager) {
    pressed.await;
    reset.toggle_configurator();
}
