use crate::hal::prelude::*;

pub(super) type Driver = <crate::hal::Wifi as crate::hal::traits::Wifi>::Driver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, expect(unused))]
pub(crate) enum Kind {
    AccessPoint,
    Station,
}

impl From<Kind> for super::Kind {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::AccessPoint => Self::DhcpClient,
            Kind::Station => Self::StaticIp,
        }
    }
}

pub(crate) type Ssid = heapless::String<32>;
pub(crate) type Password = heapless::String<64>;

#[derive(Debug)]
#[cfg_attr(test, expect(unused))]
pub(crate) struct Config {
    pub(crate) ap: Credentials,
    pub(crate) sta: Option<Credentials>,
}

#[derive(Debug)]
#[cfg_attr(test, expect(unused))]
pub(crate) struct Credentials {
    pub(crate) ssid: Ssid,
    pub(crate) password: Password,
}

pub(super) async fn init(config: crate::Config, wifi: crate::hal::Wifi) -> (Driver, Kind) {
    let config = config.network().lock(|config| {
        let ap = Credentials {
            ssid: "VerTX".try_into().unwrap(),
            password: config.ap().password().clone(),
        };

        let sta = {
            let config = config.sta();
            let ssid = config.ssid();
            let password = config.password();

            (!ssid.is_empty() && !password.is_empty()).then(|| Credentials {
                ssid: ssid.clone(),
                password: password.clone(),
            })
        };

        Config { ap, sta }
    });

    wifi.start(config).await
}
