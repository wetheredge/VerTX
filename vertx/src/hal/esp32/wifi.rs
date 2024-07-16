use embassy_executor::task;
use embassy_net::driver::Driver;
use embassy_time::Timer;
use esp_hal::clock::Clocks;
use esp_hal::peripherals;
use esp_hal::rng::Rng;
use esp_hal::timer::{ErasedTimer, PeriodicTimer};
use esp_wifi::wifi::{self, WifiApDevice, WifiController, WifiEvent, WifiStaDevice, WifiState};
use esp_wifi::EspWifiInitFor;

use crate::wifi::{Password, Ssid};

pub(super) struct GetWifi {
    pub(super) spawner: embassy_executor::Spawner,
    pub(super) clocks: Clocks<'static>,
    pub(super) rng: Rng,
    pub(super) timer: PeriodicTimer<ErasedTimer>,
    pub(super) radio_clocks: peripherals::RADIO_CLK,
    pub(super) wifi: peripherals::WIFI,
}

impl crate::hal::traits::GetWifi for GetWifi {
    const SUPPORTS_FIELD: bool = true;
    const SUPPORTS_HOME: bool = true;

    fn home(self, ssid: &'static Ssid, password: &'static Password) -> crate::hal::Wifi {
        let spawner = self.spawner;
        let init = esp_wifi::initialize(
            EspWifiInitFor::Wifi,
            self.timer,
            self.rng,
            self.radio_clocks,
            &self.clocks,
        )
        .unwrap();

        let (device, controller) = wifi::new_with_mode(&init, self.wifi, WifiStaDevice).unwrap();

        spawner.must_spawn(home_connection(controller, ssid, password));

        Device::Home(device)
    }

    fn field(self, ssid: Ssid, password: &'static Password) -> crate::hal::Wifi {
        let spawner = self.spawner;
        let init = esp_wifi::initialize(
            EspWifiInitFor::Wifi,
            self.timer,
            self.rng,
            self.radio_clocks,
            &self.clocks,
        )
        .unwrap();

        let (device, controller) = wifi::new_with_mode(&init, self.wifi, WifiApDevice).unwrap();

        spawner.must_spawn(field_connection(controller, ssid, password));

        Device::Field(device)
    }
}

enum Device {
    Home(wifi::WifiDevice<'static, WifiStaDevice>),
    Field(wifi::WifiDevice<'static, WifiApDevice>),
}

impl Driver for Device {
    type RxToken<'a> = RxToken where Self: 'a;
    type TxToken<'a> = TxToken where Self: 'a;

    fn receive(
        &mut self,
        cx: &mut core::task::Context,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        match self {
            Device::Home(wifi) => {
                Driver::receive(wifi, cx).map(|(rx, tx)| (RxToken::Home(rx), TxToken::Home(tx)))
            }
            Device::Field(wifi) => {
                Driver::receive(wifi, cx).map(|(rx, tx)| (RxToken::Field(rx), TxToken::Field(tx)))
            }
        }
    }

    fn transmit(&mut self, cx: &mut core::task::Context) -> Option<Self::TxToken<'_>> {
        match self {
            Device::Home(wifi) => Driver::transmit(wifi, cx).map(TxToken::Home),
            Device::Field(wifi) => Driver::transmit(wifi, cx).map(TxToken::Field),
        }
    }

    fn link_state(&mut self, cx: &mut core::task::Context) -> embassy_net::driver::LinkState {
        match self {
            Device::Home(wifi) => Driver::link_state(wifi, cx),
            Device::Field(wifi) => Driver::link_state(wifi, cx),
        }
    }

    fn capabilities(&self) -> embassy_net::driver::Capabilities {
        match self {
            Device::Home(wifi) => Driver::capabilities(wifi),
            Device::Field(wifi) => Driver::capabilities(wifi),
        }
    }

    fn hardware_address(&self) -> embassy_net::driver::HardwareAddress {
        match self {
            Device::Home(wifi) => Driver::hardware_address(wifi),
            Device::Field(wifi) => Driver::hardware_address(wifi),
        }
    }
}

enum RxToken {
    Home(wifi::WifiRxToken<WifiStaDevice>),
    Field(wifi::WifiRxToken<WifiApDevice>),
}

impl embassy_net::driver::RxToken for RxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        match self {
            RxToken::Home(token) => token.consume(f),
            RxToken::Field(token) => token.consume(f),
        }
    }
}

enum TxToken {
    Home(wifi::WifiTxToken<WifiStaDevice>),
    Field(wifi::WifiTxToken<WifiApDevice>),
}

impl embassy_net::driver::TxToken for TxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        match self {
            TxToken::Home(token) => token.consume(len, f),
            TxToken::Field(token) => token.consume(len, f),
        }
    }
}

#[task]
async fn home_connection(
    mut controller: WifiController<'static>,
    ssid: &'static Ssid,
    password: &'static Password,
) -> ! {
    log::info!("Starting home_connection()");

    let config = wifi::Configuration::Client(wifi::ClientConfiguration {
        ssid: ssid.clone(),
        bssid: None,
        auth_method: wifi::AuthMethod::WPA2Personal,
        password: password.clone(),
        channel: None,
    });

    loop {
        if esp_wifi::wifi::get_wifi_state() == WifiState::StaConnected {
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            log::info!("WiFi disconnected");
            Timer::after_secs(1).await;
        }

        if !matches!(controller.is_started(), Ok(true)) {
            controller.set_configuration(&config).unwrap();
            log::info!("Starting WiFi");
            controller.start().await.unwrap();
            log::info!("WiFi started");
        }

        log::info!("WiFi connecting...");
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
async fn field_connection(
    mut controller: WifiController<'static>,
    ssid: Ssid,
    _password: &'static Password,
) -> ! {
    log::info!("Starting field_connection()");

    let config = wifi::Configuration::AccessPoint(wifi::AccessPointConfiguration {
        ssid,
        ssid_hidden: false,
        channel: 1,
        secondary_channel: None,
        // auth_method: wifi::AuthMethod::WPA2Personal,
        // password: password.clone(),
        // max_connections: 1,
        ..Default::default()
    });

    loop {
        if esp_wifi::wifi::get_wifi_state() == WifiState::ApStarted {
            controller.wait_for_event(WifiEvent::ApStop).await;
            log::info!("WiFi access point stopped");
            Timer::after_secs(1).await;
        }

        if !matches!(controller.is_started(), Ok(true)) {
            controller.set_configuration(&config).unwrap();
            log::info!("Starting WiFi");
            controller.start().await.unwrap();
            log::info!("WiFi started");
        }
    }
}
