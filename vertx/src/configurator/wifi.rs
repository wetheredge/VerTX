use alloc::string::String;
use core::convert::identity;
use core::fmt;

use embassy_executor::{task, Spawner};
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::clock::Clocks;
use esp_hal::peripheral::Peripheral;
use esp_hal::{peripherals, timer};
use esp_wifi::wifi::{WifiController, WifiEvent, WifiStaDevice};
use esp_wifi::{wifi, EspWifiInitFor};
use serde::{Deserialize, Serialize};
use static_cell::make_static;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    hostname: String,
    credentials: Credentials,
}

#[derive(Clone, Serialize, Deserialize)]
struct Credentials {
    ssid: String,
    password: String,
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("ssid", &"***")
            .field("password", &"***")
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hostname: String::from("vertx"),
            credentials: Credentials {
                ssid: String::from(env!("WIFI_SSID")),
                password: String::from(env!("WIFI_PASSWORD")),
            },
        }
    }
}

pub type Stack<'d> =
    embassy_net::Stack<esp_wifi::wifi::WifiDevice<'d, esp_wifi::wifi::WifiStaDevice>>;

pub fn run(
    spawner: &Spawner,
    config: &'static crate::Config,
    clocks: &Clocks,
    timer: timer::Timer<timer::Timer0<peripherals::TIMG1>>,
    mut rng: esp_hal::Rng,
    device: impl Peripheral<P = peripherals::WIFI> + 'static,
    radio_clocks: esp_hal::system::RadioClockControl,
) -> &'static Stack<'static> {
    let init =
        esp_wifi::initialize(EspWifiInitFor::Wifi, timer, rng, radio_clocks, clocks).unwrap();

    let (wifi_interface, controller) = wifi::new_with_mode(&init, device, WifiStaDevice).unwrap();

    let mut dhcp_config = embassy_net::DhcpConfig::default();
    dhcp_config.hostname = Some(config.wifi.hostname.as_str().try_into().unwrap());

    let dhcp = embassy_net::Config::dhcpv4(dhcp_config);
    let stack = &*make_static!(embassy_net::Stack::new(
        wifi_interface,
        dhcp,
        make_static!(embassy_net::StackResources::<{ super::server::TASKS + 1 }>::new()),
        (u64::from(rng.random()) << 32) | u64::from(rng.random()),
    ));

    spawner.must_spawn(connection(controller, &config.wifi.credentials));
    spawner.must_spawn(network(stack));

    stack
}

#[task]
async fn connection(
    mut controller: WifiController<'static>,
    credentials: &'static Credentials,
) -> ! {
    log::info!("Starting connection()");

    loop {
        // If connected, wait for disconnect
        if controller.is_connected().is_ok_and(identity) {
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            log::info!("WiFi disconnected");
            Timer::after_secs(1).await;
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let config = wifi::Configuration::Client(wifi::ClientConfiguration {
                ssid: credentials.ssid.as_str().try_into().unwrap(),
                bssid: None,
                auth_method: wifi::AuthMethod::WPA2Personal,
                password: credentials.password.as_str().try_into().unwrap(),
                channel: None,
            });

            controller.set_configuration(&config).unwrap();
            log::info!("Starting WiFi");
            controller.start().await.unwrap();
            log::info!("WiFi started");
        }

        log::info!("Connecting...");
        match controller.connect().await {
            Ok(()) => log::info!("WiFi connected"),
            Err(err) => {
                log::info!("WiFi connection failed: {err:?}");
                Timer::after_secs(5).await;
            }
        }
    }
}

#[task]
async fn network(stack: &'static Stack<'static>) -> ! {
    stack.run().await
}
