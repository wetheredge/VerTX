use alloc::string::String;
use core::cell::UnsafeCell;
use core::convert::identity;
use core::fmt;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};

use embassy_executor::{task, Spawner};
use embassy_time::Timer;
use embedded_hal_async::digital::Wait;
use esp_backtrace as _;
use esp_hal::clock::Clocks;
use esp_hal::gpio::{self, AnyPin};
use esp_hal::macros::ram;
use esp_hal::peripheral::Peripheral;
use esp_hal::rtc_cntl::SocResetReason;
use esp_hal::{peripherals, timer};
use esp_wifi::wifi::{WifiController, WifiEvent, WifiStaDevice};
use esp_wifi::{wifi, EspWifiInitFor};
use serde::{Deserialize, Serialize};
use static_cell::make_static;

#[derive(Debug, Clone, Copy)]
pub struct IsEnabled {
    enabled: &'static AtomicBool,
}

impl IsEnabled {
    pub fn new() -> Self {
        static IS_SINGLETON: AtomicBool = AtomicBool::new(true);

        if !IS_SINGLETON.swap(false, Ordering::AcqRel) {
            panic!("Cannot run wifi::Enabled::new() multiple times");
        }

        // TODO: replace this with SyncUnsafeCell when it is stabilized
        struct Raw(UnsafeCell<MaybeUninit<AtomicBool>>);
        // SAFETY: the IS_SINGLETON check guarantees this only runs once and this is
        // never actually seen by multiple threads
        unsafe impl Sync for Raw {}

        #[ram(rtc_fast, uninitialized)]
        static RAW: Raw = Raw(UnsafeCell::new(MaybeUninit::uninit()));

        // Initialize on any reset other than user requested ones
        if !matches!(
            esp_hal::reset::get_reset_reason(),
            Some(
                SocResetReason::CoreSw | SocResetReason::CoreUsbUart | SocResetReason::CoreUsbJtag
            )
        ) {
            // SAFETY: IS_SINGLETON guarantees this can only run once, therefore this
            // mutable reference is always unique
            unsafe { (*RAW.0.get()).write(AtomicBool::new(false)) };
        }

        // SAFETY: IS_SINGLETON check guarantees this only runs once. The previous
        // &mut is contained in the scope of the if, leaving this the sole reference
        let raw = unsafe { &*RAW.0.get() };

        // SAFETY: already been initialized by the if above, or on a previous boot
        let enabled = unsafe { raw.assume_init_ref() };

        Self { enabled }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub fn toggle(&self) {
        self.enabled.fetch_xor(true, Ordering::AcqRel);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    hostname: String,
    credentials: Credentials,
}

#[derive(Serialize, Deserialize)]
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
            hostname: String::from("vhs"),
            credentials: Credentials {
                ssid: String::from(env!("WIFI_SSID")),
                password: String::from(env!("WIFI_PASSWORD")),
            },
        }
    }
}

pub type Stack<'d> =
    embassy_net::Stack<esp_wifi::wifi::WifiDevice<'d, esp_wifi::wifi::WifiStaDevice>>;

#[allow(clippy::too_many_arguments)]
pub fn run(
    spawner: &Spawner,
    config: &'static crate::Config,
    clocks: &Clocks,
    timer: timer::Timer<timer::Timer0<peripherals::TIMG1>>,
    mut rng: esp_hal::Rng,
    device: impl Peripheral<P = peripherals::WIFI> + 'static,
    radio_clocks: esp_hal::system::RadioClockControl,
    mode: crate::mode::Publisher<'static>,
) -> &'static Stack<'static> {
    mode.publish(crate::Mode::PreWiFi);

    let init =
        esp_wifi::initialize(EspWifiInitFor::Wifi, timer, rng, radio_clocks, clocks).unwrap();

    let (wifi_interface, controller) = wifi::new_with_mode(&init, device, WifiStaDevice).unwrap();

    let mut dhcp_config = embassy_net::DhcpConfig::default();
    dhcp_config.hostname = Some(config.wifi.hostname.as_str().try_into().unwrap());

    let dhcp = embassy_net::Config::dhcpv4(dhcp_config);
    let stack = &*make_static!(embassy_net::Stack::new(
        wifi_interface,
        dhcp,
        make_static!(embassy_net::StackResources::<{ crate::server::TASKS + 1 }>::new()),
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

#[task]
pub async fn toggle_button(mut button: AnyPin<gpio::Input<gpio::PullUp>>, enabled: IsEnabled) {
    button.wait_for_falling_edge().await.unwrap();
    enabled.toggle();
    esp_hal::reset::software_reset();
}
