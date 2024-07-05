use core::convert::identity;
use core::fmt;

use embassy_executor::{task, Spawner};
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::clock::Clocks;
use esp_hal::peripherals;
use esp_hal::timer::timg;
use esp_wifi::wifi::{WifiController, WifiEvent, WifiStaDevice};
use esp_wifi::{wifi, EspWifiInitFor};
use heapless::String;
use static_cell::make_static;

pub type Config = vertx_config::BootSnapshot<RawConfig, crate::mutex::SingleCore>;

#[derive(Clone, vertx_config::UpdateMut, vertx_config::Storage)]
pub struct RawConfig {
    hostname: String<32>,
    password: String<64>,
    home_ssid: String<32>,
    home_password: String<64>,
}

impl fmt::Debug for RawConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawConfig")
            .field("hostname", &self.hostname)
            .field("password", &"***")
            .field("home_ssid", &"***")
            .field("home_password", &"***")
            .finish()
    }
}

impl Default for RawConfig {
    fn default() -> Self {
        Self {
            hostname: "vertx".try_into().unwrap(),
            password: String::new(), // TODO
            home_ssid: env!("WIFI_SSID").try_into().unwrap(),
            home_password: env!("WIFI_PASSWORD").try_into().unwrap(),
        }
    }
}

impl RawConfig {
    fn valid_home(&self) -> bool {
        !self.home_ssid.is_empty() && !self.home_password.is_empty()
    }
}

pub type Stack<'d> =
    embassy_net::Stack<esp_wifi::wifi::WifiDevice<'d, esp_wifi::wifi::WifiStaDevice>>;

pub fn run(
    spawner: &Spawner,
    config: &'static crate::Config,
    clocks: &Clocks,
    timer: timg::Timer<timg::Timer0<peripherals::TIMG1>, esp_hal::Blocking>,
    mut rng: esp_hal::rng::Rng,
    device: peripherals::WIFI,
    radio_clocks: peripherals::RADIO_CLK,
) -> &'static Stack<'static> {
    let config = config.wifi.boot();

    let init =
        esp_wifi::initialize(EspWifiInitFor::Wifi, timer, rng, radio_clocks, clocks).unwrap();

    let (wifi_interface, controller) = wifi::new_with_mode(&init, device, WifiStaDevice).unwrap();

    let mut dhcp_config = embassy_net::DhcpConfig::default();
    dhcp_config.hostname = Some(config.hostname.clone());

    let dhcp = embassy_net::Config::dhcpv4(dhcp_config);
    let stack = &*make_static!(embassy_net::Stack::new(
        wifi_interface,
        dhcp,
        make_static!(embassy_net::StackResources::<{ super::server::TASKS + 1 }>::new()),
        (u64::from(rng.random()) << 32) | u64::from(rng.random()),
    ));

    spawner.must_spawn(connection(controller, config));
    spawner.must_spawn(network(stack));

    stack
}

#[task]
async fn connection(mut controller: WifiController<'static>, config: &'static RawConfig) -> ! {
    log::info!("Starting connection()");

    assert!(config.valid_home());

    loop {
        // If connected, wait for disconnect
        if controller.is_connected().is_ok_and(identity) {
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            log::info!("WiFi disconnected");
            Timer::after_secs(1).await;
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let config = wifi::Configuration::Client(wifi::ClientConfiguration {
                ssid: config.home_ssid.clone(),
                bssid: None,
                auth_method: wifi::AuthMethod::WPA2Personal,
                password: config.home_password.clone(),
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
