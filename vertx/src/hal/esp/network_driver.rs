use core::task::Context;

use esp_wifi::wifi::{self, WifiApDevice, WifiStaDevice};

pub(super) enum Driver {
    Sta(wifi::WifiDevice<'static, WifiStaDevice>),
    Ap(wifi::WifiDevice<'static, WifiApDevice>),
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
            Self::Sta(wifi) => {
                Driver::receive(wifi, cx).map(|(rx, tx)| (RxToken::Sta(rx), TxToken::Sta(tx)))
            }
            Self::Ap(wifi) => {
                Driver::receive(wifi, cx).map(|(rx, tx)| (RxToken::Ap(rx), TxToken::Ap(tx)))
            }
        }
    }

    fn transmit(&mut self, cx: &mut Context) -> Option<Self::TxToken<'_>> {
        use embassy_net::driver::Driver;
        match self {
            Self::Sta(wifi) => Driver::transmit(wifi, cx).map(TxToken::Sta),
            Self::Ap(wifi) => Driver::transmit(wifi, cx).map(TxToken::Ap),
        }
    }

    fn link_state(&mut self, cx: &mut Context) -> embassy_net::driver::LinkState {
        use embassy_net::driver::Driver;
        match self {
            Self::Sta(wifi) => Driver::link_state(wifi, cx),
            Self::Ap(wifi) => Driver::link_state(wifi, cx),
        }
    }

    fn capabilities(&self) -> embassy_net::driver::Capabilities {
        use embassy_net::driver::Driver;
        match self {
            Self::Sta(wifi) => Driver::capabilities(wifi),
            Self::Ap(wifi) => Driver::capabilities(wifi),
        }
    }

    fn hardware_address(&self) -> embassy_net::driver::HardwareAddress {
        use embassy_net::driver::Driver;
        match self {
            Self::Sta(wifi) => Driver::hardware_address(wifi),
            Self::Ap(wifi) => Driver::hardware_address(wifi),
        }
    }
}

pub(super) enum RxToken {
    Sta(wifi::WifiRxToken<WifiStaDevice>),
    Ap(wifi::WifiRxToken<WifiApDevice>),
}

impl embassy_net::driver::RxToken for RxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        match self {
            RxToken::Sta(token) => token.consume(f),
            RxToken::Ap(token) => token.consume(f),
        }
    }
}

pub(super) enum TxToken {
    Sta(wifi::WifiTxToken<WifiStaDevice>),
    Ap(wifi::WifiTxToken<WifiApDevice>),
}

impl embassy_net::driver::TxToken for TxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        match self {
            TxToken::Sta(token) => token.consume(len, f),
            TxToken::Ap(token) => token.consume(len, f),
        }
    }
}
