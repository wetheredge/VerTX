#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]
#![expect(missing_debug_implementations)]

use embassy_executor::{task, Spawner};
use embassy_time::Timer;
use esp_hal::peripherals;
use esp_hal::rng::Rng;
use esp_hal::timer::AnyTimer;
use esp_wifi::wifi::{self, WifiApDevice, WifiController, WifiEvent, WifiStaDevice, WifiState};
use esp_wifi::EspWifiController;
use static_cell::StaticCell;
use vertx_network::{Password, Ssid};

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

    const SUPPORTS_FIELD: bool = true;
    const SUPPORTS_HOME: bool = true;

    fn home(self, ssid: Ssid, password: Password) -> Self::Driver {
        static CONTROLLER: StaticCell<EspWifiController> = StaticCell::new();
        let controller =
            CONTROLLER.init(esp_wifi::init(self.timer, self.rng, self.radio_clocks).unwrap());

        let (device, wifi_controller) =
            wifi::new_with_mode(controller, self.wifi, WifiStaDevice).unwrap();

        self.spawner
            .must_spawn(home_connection(wifi_controller, ssid, password));

        Driver::Home(device)
    }

    fn field(self, ssid: Ssid, password: Password) -> Self::Driver {
        static CONTROLLER: StaticCell<EspWifiController> = StaticCell::new();
        let controller =
            CONTROLLER.init(esp_wifi::init(self.timer, self.rng, self.radio_clocks).unwrap());

        let (device, wifi_controller) =
            wifi::new_with_mode(controller, self.wifi, WifiApDevice).unwrap();

        self.spawner
            .must_spawn(field_connection(wifi_controller, ssid, password));

        Driver::Field(device)
    }
}

pub enum Driver {
    Home(wifi::WifiDevice<'static, WifiStaDevice>),
    Field(wifi::WifiDevice<'static, WifiApDevice>),
}

impl embassy_net::driver::Driver for Driver {
    type RxToken<'a> = RxToken where Self: 'a;
    type TxToken<'a> = TxToken where Self: 'a;

    fn receive(
        &mut self,
        cx: &mut core::task::Context,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        match self {
            Self::Home(wifi) => embassy_net::driver::Driver::receive(wifi, cx)
                .map(|(rx, tx)| (RxToken::Home(rx), TxToken::Home(tx))),
            Self::Field(wifi) => embassy_net::driver::Driver::receive(wifi, cx)
                .map(|(rx, tx)| (RxToken::Field(rx), TxToken::Field(tx))),
        }
    }

    fn transmit(&mut self, cx: &mut core::task::Context) -> Option<Self::TxToken<'_>> {
        match self {
            Self::Home(wifi) => embassy_net::driver::Driver::transmit(wifi, cx).map(TxToken::Home),
            Self::Field(wifi) => {
                embassy_net::driver::Driver::transmit(wifi, cx).map(TxToken::Field)
            }
        }
    }

    fn link_state(&mut self, cx: &mut core::task::Context) -> embassy_net::driver::LinkState {
        match self {
            Self::Home(wifi) => embassy_net::driver::Driver::link_state(wifi, cx),
            Self::Field(wifi) => embassy_net::driver::Driver::link_state(wifi, cx),
        }
    }

    fn capabilities(&self) -> embassy_net::driver::Capabilities {
        match self {
            Self::Home(wifi) => embassy_net::driver::Driver::capabilities(wifi),
            Self::Field(wifi) => embassy_net::driver::Driver::capabilities(wifi),
        }
    }

    fn hardware_address(&self) -> embassy_net::driver::HardwareAddress {
        match self {
            Self::Home(wifi) => embassy_net::driver::Driver::hardware_address(wifi),
            Self::Field(wifi) => embassy_net::driver::Driver::hardware_address(wifi),
        }
    }
}

pub enum RxToken {
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

pub enum TxToken {
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
    ssid: Ssid,
    password: Password,
) -> ! {
    log::info!("Starting home_connection()");

    let config = wifi::Configuration::Client(wifi::ClientConfiguration {
        ssid,
        bssid: None,
        auth_method: wifi::AuthMethod::WPA2Personal,
        password,
        channel: None,
    });

    loop {
        if esp_wifi::wifi::wifi_state() == WifiState::StaConnected {
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            log::info!("WiFi disconnected");
            Timer::after_secs(1).await;
        }

        if !matches!(controller.is_started(), Ok(true)) {
            controller.set_configuration(&config).unwrap();
            log::info!("Starting WiFi");
            controller.start_async().await.unwrap();
            log::info!("WiFi started");
        }

        log::info!("WiFi connecting...");
        match controller.connect_async().await {
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
    _password: Password,
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
        if esp_wifi::wifi::wifi_state() == WifiState::ApStarted {
            controller.wait_for_event(WifiEvent::ApStop).await;
            log::info!("WiFi access point stopped");
            Timer::after_secs(1).await;
        }

        if !matches!(controller.is_started(), Ok(true)) {
            controller.set_configuration(&config).unwrap();
            log::info!("Starting WiFi");
            controller.start_async().await.unwrap();
            log::info!("WiFi started");
        }
    }
}
