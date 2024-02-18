use core::convert::identity;

use embassy_executor::{task, Spawner};
use embassy_time::Timer;
use embedded_svc::wifi::Wifi;
use esp_backtrace as _;
use esp_wifi::wifi::{WifiController, WifiEvent, WifiStaDevice};
use esp_wifi::{wifi, EspWifiInitFor};
use hal::clock::Clocks;
use hal::peripheral::Peripheral;
use hal::{peripherals, timer};
use static_cell::make_static;

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");
const HOSTNAME: &str = "vhs";

pub type Stack<'d> =
    embassy_net::Stack<esp_wifi::wifi::WifiDevice<'d, esp_wifi::wifi::WifiStaDevice>>;

pub fn run(
    spawner: &Spawner,
    clocks: &Clocks,
    timer: timer::Timer<timer::Timer0<peripherals::TIMG1>>,
    mut rng: hal::Rng,
    device: impl Peripheral<P = peripherals::WIFI> + 'static,
    radio_clocks: hal::system::RadioClockControl,
    status: crate::status::Publisher<'static>,
) -> &'static Stack<'static> {
    status.publish(crate::Status::PreWiFi);

    let init =
        esp_wifi::initialize(EspWifiInitFor::Wifi, timer, rng, radio_clocks, clocks).unwrap();

    let (wifi_interface, controller) = wifi::new_with_mode(&init, device, WifiStaDevice).unwrap();

    let mut dhcp_config = embassy_net::DhcpConfig::default();
    dhcp_config.hostname = Some(HOSTNAME.try_into().unwrap());

    let dhcp = embassy_net::Config::dhcpv4(dhcp_config);
    let stack = &*make_static!(embassy_net::Stack::new(
        wifi_interface,
        dhcp,
        make_static!(embassy_net::StackResources::<{ crate::server::TASKS + 1 }>::new()),
        (u64::from(rng.random()) << 32) | u64::from(rng.random()),
    ));

    spawner.must_spawn(connection(controller));
    spawner.must_spawn(network(stack));

    stack
}

#[task]
async fn connection(mut controller: WifiController<'static>) -> ! {
    use embedded_svc::wifi;

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
                ssid: WIFI_SSID.try_into().unwrap(),
                bssid: None,
                auth_method: wifi::AuthMethod::WPA2Personal,
                password: WIFI_PASSWORD.try_into().unwrap(),
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
