use core::task;

use embassy_net::driver;

pub(super) enum Driver {
    #[cfg(feature = "network-usb-ethernet")]
    Ethernet(crate::usb::ncm_cdc::NetDriver),
    #[cfg(feature = "network-wifi")]
    Wifi(super::wifi::Driver),
}

pub(super) enum RxToken<'a> {
    #[cfg(feature = "network-usb-ethernet")]
    Ethernet(<crate::usb::ncm_cdc::NetDriver as driver::Driver>::RxToken<'a>),
    #[cfg(feature = "network-wifi")]
    Wifi(<super::wifi::Driver as driver::Driver>::RxToken<'a>),
}

pub(super) enum TxToken<'a> {
    #[cfg(feature = "network-usb-ethernet")]
    Ethernet(<crate::usb::ncm_cdc::NetDriver as driver::Driver>::TxToken<'a>),
    #[cfg(feature = "network-wifi")]
    Wifi(<super::wifi::Driver as driver::Driver>::TxToken<'a>),
}

impl driver::Driver for Driver {
    type RxToken<'a> = RxToken<'a>;
    type TxToken<'a> = TxToken<'a>;

    delegate::delegate! {
        to match self {
            #[cfg(feature = "network-usb-ethernet")]
            Self::Ethernet(driver) => driver,
            #[cfg(feature = "network-wifi")]
            Self::Wifi(driver) => driver,
        } {
            fn link_state(&mut self, cx: &mut task::Context) -> driver::LinkState;
            fn capabilities(&self) -> driver::Capabilities;
            fn hardware_address(&self) -> driver::HardwareAddress;
        }
    }

    fn receive(
        &mut self,
        cx: &mut task::Context,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        match self {
            #[cfg(feature = "network-usb-ethernet")]
            Self::Ethernet(driver) => driver
                .receive(cx)
                .map(|(rx, tx)| (RxToken::Ethernet(rx), TxToken::Ethernet(tx))),
            #[cfg(feature = "network-wifi")]
            Self::Wifi(driver) => driver
                .receive(cx)
                .map(|(rx, tx)| (RxToken::Wifi(rx), TxToken::Wifi(tx))),
        }
    }

    fn transmit(&mut self, cx: &mut task::Context) -> Option<Self::TxToken<'_>> {
        match self {
            #[cfg(feature = "network-usb-ethernet")]
            Self::Ethernet(driver) => driver.transmit(cx).map(TxToken::Ethernet),
            #[cfg(feature = "network-wifi")]
            Self::Wifi(driver) => driver.transmit(cx).map(TxToken::Wifi),
        }
    }
}

impl driver::RxToken for RxToken<'_> {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        match self {
            #[cfg(feature = "network-usb-ethernet")]
            Self::Ethernet(driver) => driver.consume(f),
            #[cfg(feature = "network-wifi")]
            Self::Wifi(driver) => driver.consume(f),
        }
    }
}

impl driver::TxToken for TxToken<'_> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        match self {
            #[cfg(feature = "network-usb-ethernet")]
            Self::Ethernet(driver) => driver.consume(len, f),
            #[cfg(feature = "network-wifi")]
            Self::Wifi(wifi) => wifi.consume(len, f),
        }
    }
}
