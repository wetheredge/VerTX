#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]
#![expect(missing_debug_implementations)]

mod driver;

use embassy_executor::{Spawner, task};
use embassy_time::{Duration, Timer, with_timeout};
use esp_hal::peripherals;
use esp_hal::rng::Rng;
use esp_hal::timer::AnyTimer;
use esp_wifi::EspWifiController;
use esp_wifi::wifi::{self, WifiController, WifiError, WifiEvent, WifiState};
use static_cell::StaticCell;
use vertx_network::{Credentials, NetworkKind};

pub use self::driver::Driver;

pub struct Hal {
    spawner: Spawner,
    rng: Rng,
    timer: AnyTimer,
    radio_clocks: peripherals::RADIO_CLK,
    wifi: peripherals::WIFI,
}

impl Hal {
    pub fn new(
        spawner: Spawner,
        rng: Rng,
        timer: AnyTimer,
        radio_clocks: peripherals::RADIO_CLK,
        wifi: peripherals::WIFI,
    ) -> Self {
        Self {
            spawner,
            rng,
            timer,
            radio_clocks,
            wifi,
        }
    }
}

impl vertx_network::Hal for Hal {
    type Driver = Driver;

    async fn start(
        self,
        home: Option<Credentials>,
        field: Credentials,
    ) -> (NetworkKind, Self::Driver) {
        static CONTROLLER: StaticCell<EspWifiController> = StaticCell::new();
        let initted =
            CONTROLLER.init(esp_wifi::init(self.timer, self.rng, self.radio_clocks).unwrap());

        let (ap, sta, mut controller) = wifi::new_ap_sta(initted, self.wifi).unwrap();

        let mut home_connected = false;
        if let Some(home) = home {
            match connect_to_home(&mut controller, home).await {
                Ok(connected) => home_connected = connected,
                Err(err) => loog::error!("Failed to join home wifi: {err:?}"),
            }
        }

        let (network, driver) = if home_connected {
            (NetworkKind::Home, Driver::Home(sta))
        } else {
            // Failed to connect to home network, start AP instead

            let ap_config = wifi::AccessPointConfiguration {
                ssid: field.ssid,
                ssid_hidden: false,
                channel: 1,
                secondary_channel: None,
                // auth_method: wifi::AuthMethod::WPA2Personal,
                // password: password.clone(),
                // max_connections: 1,
                ..Default::default()
            };

            controller
                .set_configuration(&wifi::Configuration::AccessPoint(ap_config))
                .unwrap();

            // AP will get started by `connection()` below
            (NetworkKind::Field, Driver::Field(ap))
        };

        self.spawner.must_spawn(connection(controller, network));
        (network, driver)
    }
}

async fn connect_to_home(
    controller: &mut WifiController<'_>,
    credentials: Credentials,
) -> Result<bool, WifiError> {
    loog::info!(
        "Trying to connect to home wifi: {=str:?}",
        &credentials.ssid
    );

    let config = wifi::ClientConfiguration {
        ssid: credentials.ssid,
        bssid: None,
        auth_method: wifi::AuthMethod::WPA2Personal,
        password: credentials.password,
        channel: None,
    };

    controller.set_configuration(&wifi::Configuration::Client(config))?;
    controller.start_async().await?;

    let connecting = with_timeout(Duration::from_secs(30), controller.connect_async());
    let outcome = match connecting.await {
        // Early return to skip controller.stop()
        Ok(Ok(())) => return Ok(true),
        Err(embassy_time::TimeoutError) => {
            loog::warn!("Failed to connect to home wifi in time");
            Ok(false)
        }
        Ok(Err(err)) => Err(err),
    };
    controller.stop_async().await?;
    outcome
}

#[task]
async fn connection(mut controller: WifiController<'static>, network: NetworkKind) {
    match network {
        NetworkKind::Home => loop {
            if wifi::sta_state() == WifiState::StaConnected {
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                loog::info!("WiFi disconnected");
            }

            while let Err(err) = controller.connect_async().await {
                loog::error!("Failed to rejoin wifi: {err:?}");
                Timer::after_secs(5).await;
            }
        },

        NetworkKind::Field => loop {
            if wifi::ap_state() == WifiState::ApStarted {
                controller.wait_for_event(WifiEvent::ApStop).await;
                Timer::after_secs(1).await;
            }

            while let Err(err) = controller.start_async().await {
                loog::error!("Failed to restart wifi: {err:?}");
                Timer::after_secs(1).await;
            }
        },
    }
}
