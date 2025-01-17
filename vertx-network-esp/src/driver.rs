use core::task::Context;

use esp_wifi::wifi::{self, WifiApDevice, WifiStaDevice};

pub enum Driver {
    Home(wifi::WifiDevice<'static, WifiStaDevice>),
    Field(wifi::WifiDevice<'static, WifiApDevice>),
}

impl embassy_net::driver::Driver for Driver {
    type RxToken<'a>
        = RxToken
    where
        Self: 'a;
    type TxToken<'a>
        = TxToken
    where
        Self: 'a;

    fn receive(&mut self, cx: &mut Context) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        use embassy_net::driver::Driver;
        match self {
            Self::Home(wifi) => {
                Driver::receive(wifi, cx).map(|(rx, tx)| (RxToken::Home(rx), TxToken::Home(tx)))
            }
            Self::Field(wifi) => {
                Driver::receive(wifi, cx).map(|(rx, tx)| (RxToken::Field(rx), TxToken::Field(tx)))
            }
        }
    }

    fn transmit(&mut self, cx: &mut Context) -> Option<Self::TxToken<'_>> {
        use embassy_net::driver::Driver;
        match self {
            Self::Home(wifi) => Driver::transmit(wifi, cx).map(TxToken::Home),
            Self::Field(wifi) => Driver::transmit(wifi, cx).map(TxToken::Field),
        }
    }

    fn link_state(&mut self, cx: &mut Context) -> embassy_net::driver::LinkState {
        use embassy_net::driver::Driver;
        match self {
            Self::Home(wifi) => Driver::link_state(wifi, cx),
            Self::Field(wifi) => Driver::link_state(wifi, cx),
        }
    }

    fn capabilities(&self) -> embassy_net::driver::Capabilities {
        use embassy_net::driver::Driver;
        match self {
            Self::Home(wifi) => Driver::capabilities(wifi),
            Self::Field(wifi) => Driver::capabilities(wifi),
        }
    }

    fn hardware_address(&self) -> embassy_net::driver::HardwareAddress {
        use embassy_net::driver::Driver;
        match self {
            Self::Home(wifi) => Driver::hardware_address(wifi),
            Self::Field(wifi) => Driver::hardware_address(wifi),
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
