use alloc::string::ToString as _;

use embassy_executor::{Spawner, task};
use embassy_time::{Duration, Timer, with_timeout};
use esp_hal::peripherals;
use esp_hal::rng::Rng;
use esp_hal::timer::AnyTimer;
use esp_wifi::EspWifiController;
use esp_wifi::wifi::{self, WifiController, WifiError, WifiEvent, WifiState};
use static_cell::StaticCell;

use crate::network::wifi::{Config, Credentials, Kind};

pub(super) struct Wifi {
    pub(super) spawner: Spawner,
    pub(super) rng: Rng,
    pub(super) timer: AnyTimer<'static>,
    pub(super) wifi: peripherals::WIFI<'static>,
}

impl crate::hal::traits::Wifi for Wifi {
    type Driver = wifi::WifiDevice<'static>;

    async fn start(self, config: Config) -> (Self::Driver, Kind) {
        static CONTROLLER: StaticCell<EspWifiController> = StaticCell::new();
        let initted = CONTROLLER.init(esp_wifi::init(self.timer, self.rng).unwrap());

        let (mut controller, interfaces) = wifi::new(initted, self.wifi).unwrap();

        let mut home_connected = false;
        if let Some(sta) = config.sta {
            match connect_to_sta(&mut controller, sta).await {
                Ok(connected) => home_connected = connected,
                Err(err) => loog::error!("Failed to join home wifi: {err:?}"),
            }
        }

        let (kind, driver) = if home_connected {
            (Kind::Station, interfaces.sta)
        } else {
            // Failed to connect to home network, start AP instead

            let ap_config = wifi::AccessPointConfiguration {
                ssid: config.ap.ssid.to_string(),
                ssid_hidden: false,
                channel: 1,
                secondary_channel: None,
                // auth_method: wifi::AuthMethod::WPA2Personal,
                // FIXME: set password
                // password: config.ap.password.clone(),
                // max_connections: 1,
                ..Default::default()
            };

            controller
                .set_configuration(&wifi::Configuration::AccessPoint(ap_config))
                .unwrap();

            // AP will get started by `connection()` below
            (Kind::AccessPoint, interfaces.ap)
        };

        self.spawner.must_spawn(connection(controller, kind));
        (driver, kind)
    }
}

async fn connect_to_sta(
    controller: &mut WifiController<'_>,
    credentials: Credentials,
) -> Result<bool, WifiError> {
    loog::info!(
        "Trying to connect to network with SSID {=str:?}",
        &credentials.ssid
    );

    let config = wifi::ClientConfiguration {
        ssid: credentials.ssid.to_string(),
        bssid: None,
        auth_method: wifi::AuthMethod::WPA2Personal,
        password: credentials.password.to_string(),
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
async fn connection(mut controller: WifiController<'static>, kind: Kind) {
    match kind {
        Kind::Station => loop {
            if wifi::sta_state() == WifiState::StaConnected {
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                loog::info!("WiFi disconnected");
            }

            while let Err(err) = controller.connect_async().await {
                loog::error!("Failed to rejoin wifi: {err:?}");
                Timer::after_secs(5).await;
            }
        },

        Kind::AccessPoint => loop {
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
