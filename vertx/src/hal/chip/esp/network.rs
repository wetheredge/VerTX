use alloc::string::ToString as _;

use embassy_executor::{Spawner, task};
use embassy_time::{Duration, Timer, with_timeout};
use esp_hal::peripherals;
use esp_hal::rng::Rng;
use esp_radio::wifi::{self, WifiApState, WifiController, WifiError, WifiEvent, WifiStaState};
use static_cell::StaticCell;

use crate::network::{Credentials, Kind};

pub(super) struct Network {
    pub(super) spawner: Spawner,
    pub(super) rng: Rng,
    pub(super) wifi: peripherals::WIFI<'static>,
}

impl crate::hal::traits::Network for Network {
    type Driver = wifi::WifiDevice<'static>;

    fn seed(&mut self) -> u64 {
        let upper = u64::from(self.rng.random()) << 32;
        let lower = u64::from(self.rng.random());

        upper | lower
    }

    async fn start(
        self,
        sta: Option<crate::network::Credentials>,
        ap: crate::network::Credentials,
    ) -> (Kind, Self::Driver) {
        static CONTROLLER: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
        let initted = CONTROLLER.init(esp_radio::init().unwrap());

        let (mut controller, interfaces) =
            wifi::new(initted, self.wifi, Default::default()).unwrap();

        let mut home_connected = false;
        if let Some(sta) = sta {
            match connect_to_sta(&mut controller, sta).await {
                Ok(connected) => home_connected = connected,
                Err(err) => loog::error!("Failed to join home wifi: {err:?}"),
            }
        }

        let (network, driver) = if home_connected {
            (Kind::Station, interfaces.sta)
        } else {
            // Failed to connect to home network, start AP instead

            let ap_config = wifi::AccessPointConfig::default()
                .with_ssid(ap.ssid.to_string())
                .with_channel(1);

            controller
                .set_config(&wifi::ModeConfig::AccessPoint(ap_config))
                .unwrap();

            // AP will get started by `connection()` below
            (Kind::AccessPoint, interfaces.ap)
        };

        self.spawner.must_spawn(connection(controller, network));
        (network, driver)
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

    let config = wifi::ClientConfig::default()
        .with_ssid(credentials.ssid.to_string())
        .with_auth_method(wifi::AuthMethod::Wpa2Personal)
        .with_password(credentials.password.to_string());

    controller.set_config(&wifi::ModeConfig::Client(config))?;
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
            if wifi::sta_state() == WifiStaState::Connected {
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                loog::info!("WiFi disconnected");
            }

            while let Err(err) = controller.connect_async().await {
                loog::error!("Failed to rejoin wifi: {err:?}");
                Timer::after_secs(5).await;
            }
        },

        Kind::AccessPoint => loop {
            if wifi::ap_state() == WifiApState::Started {
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
