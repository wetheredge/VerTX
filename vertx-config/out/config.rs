#[derive(Debug, ::serde::Deserialize, ::serde::Serialize)]
#[allow(non_snake_case)]
pub(crate) struct RawConfig {
    pub(super) name: ::heapless::String<20>,
    pub(super) leds_brightness: u8,
    pub(super) network_hostname: ::heapless::String<32>,
    pub(super) network_ap_password: ::heapless::String<64>,
    pub(super) network_sta_ssid: ::heapless::String<32>,
    pub(super) network_sta_password: ::heapless::String<64>,
}

#[allow(clippy::derivable_impls)]
impl Default for RawConfig {
    fn default() -> Self {
        Self {
            name: "VerTX".try_into().unwrap(),
            leds_brightness: 10,
            network_hostname: "vertx".try_into().unwrap(),
            network_ap_password: Default::default(),
            network_sta_ssid: Default::default(),
            network_sta_password: Default::default(),
        }
    }
}
pub(crate) const BYTE_LENGTH: usize = 242;
#[allow(non_camel_case_types, unused)]
pub(super) mod key {
    #[derive(Clone, Copy)]
    pub(crate) struct Root;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Name;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Leds;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Leds_Brightness;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network_Hostname;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network_Ap;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network_Ap_Password;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network_Sta;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network_Sta_Ssid;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network_Sta_Password;
}

#[allow(unused)]
impl super::View<key::Root> {
    pub(crate) fn lock<T>(&self, f: impl FnOnce(super::LockedView<'_, key::Root>) -> T) -> T {
        self.manager.state.lock(|state| {
            f(super::LockedView {
                config: &state.borrow().config,
                _key: ::core::marker::PhantomData,
            })
        })
    }

    pub(crate) fn name(&self) -> super::View<key::Root_Name> {
        super::View {
            manager: self.manager,
            _key: ::core::marker::PhantomData,
        }
    }

    pub(crate) fn leds(&self) -> super::View<key::Root_Leds> {
        super::View {
            manager: self.manager,
            _key: ::core::marker::PhantomData,
        }
    }

    pub(crate) fn network(&self) -> super::View<key::Root_Network> {
        super::View {
            manager: self.manager,
            _key: ::core::marker::PhantomData,
        }
    }
}

#[allow(unused)]
impl super::LockedView<'_, key::Root> {
    pub(crate) fn name(&self) -> super::LockedView<'_, key::Root_Name> {
        super::LockedView {
            config: self.config,
            _key: ::core::marker::PhantomData,
        }
    }

    pub(crate) fn leds(&self) -> super::LockedView<'_, key::Root_Leds> {
        super::LockedView {
            config: self.config,
            _key: ::core::marker::PhantomData,
        }
    }

    pub(crate) fn network(&self) -> super::LockedView<'_, key::Root_Network> {
        super::LockedView {
            config: self.config,
            _key: ::core::marker::PhantomData,
        }
    }
}

#[allow(unused)]
impl super::View<key::Root_Name> {
    pub(crate) fn lock<T>(&self, f: impl FnOnce(&::heapless::String<20>) -> T) -> T {
        self.manager
            .state
            .lock(|state| f(&state.borrow().config.name))
    }

    pub(crate) fn subscribe(&self) -> Option<super::Subscriber> {
        self.manager.subscribe(0)
    }
}

impl ::core::ops::Deref for super::LockedView<'_, key::Root_Name> {
    type Target = ::heapless::String<20>;

    fn deref(&self) -> &Self::Target {
        &self.config.name
    }
}

#[allow(unused)]
impl super::View<key::Root_Leds> {
    pub(crate) fn lock<T>(&self, f: impl FnOnce(super::LockedView<'_, key::Root_Leds>) -> T) -> T {
        self.manager.state.lock(|state| {
            f(super::LockedView {
                config: &state.borrow().config,
                _key: ::core::marker::PhantomData,
            })
        })
    }

    pub(crate) fn brightness(&self) -> super::View<key::Root_Leds_Brightness> {
        super::View {
            manager: self.manager,
            _key: ::core::marker::PhantomData,
        }
    }
}

#[allow(unused)]
impl super::LockedView<'_, key::Root_Leds> {
    pub(crate) fn brightness(&self) -> super::LockedView<'_, key::Root_Leds_Brightness> {
        super::LockedView {
            config: self.config,
            _key: ::core::marker::PhantomData,
        }
    }
}

#[allow(unused)]
impl super::View<key::Root_Leds_Brightness> {
    pub(crate) fn lock<T>(&self, f: impl FnOnce(&u8) -> T) -> T {
        self.manager
            .state
            .lock(|state| f(&state.borrow().config.leds_brightness))
    }

    pub(crate) fn subscribe(&self) -> Option<super::Subscriber> {
        self.manager.subscribe(1)
    }
}

impl ::core::ops::Deref for super::LockedView<'_, key::Root_Leds_Brightness> {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.config.leds_brightness
    }
}

#[allow(unused)]
impl super::View<key::Root_Network> {
    pub(crate) fn lock<T>(
        &self,
        f: impl FnOnce(super::LockedView<'_, key::Root_Network>) -> T,
    ) -> T {
        self.manager.state.lock(|state| {
            f(super::LockedView {
                config: &state.borrow().config,
                _key: ::core::marker::PhantomData,
            })
        })
    }

    pub(crate) fn hostname(&self) -> super::View<key::Root_Network_Hostname> {
        super::View {
            manager: self.manager,
            _key: ::core::marker::PhantomData,
        }
    }

    pub(crate) fn ap(&self) -> super::View<key::Root_Network_Ap> {
        super::View {
            manager: self.manager,
            _key: ::core::marker::PhantomData,
        }
    }

    pub(crate) fn sta(&self) -> super::View<key::Root_Network_Sta> {
        super::View {
            manager: self.manager,
            _key: ::core::marker::PhantomData,
        }
    }
}

#[allow(unused)]
impl super::LockedView<'_, key::Root_Network> {
    pub(crate) fn hostname(&self) -> super::LockedView<'_, key::Root_Network_Hostname> {
        super::LockedView {
            config: self.config,
            _key: ::core::marker::PhantomData,
        }
    }

    pub(crate) fn ap(&self) -> super::LockedView<'_, key::Root_Network_Ap> {
        super::LockedView {
            config: self.config,
            _key: ::core::marker::PhantomData,
        }
    }

    pub(crate) fn sta(&self) -> super::LockedView<'_, key::Root_Network_Sta> {
        super::LockedView {
            config: self.config,
            _key: ::core::marker::PhantomData,
        }
    }
}

#[allow(unused)]
impl super::View<key::Root_Network_Hostname> {
    pub(crate) fn lock<T>(&self, f: impl FnOnce(&::heapless::String<32>) -> T) -> T {
        self.manager
            .state
            .lock(|state| f(&state.borrow().config.network_hostname))
    }

    pub(crate) fn subscribe(&self) -> Option<super::Subscriber> {
        self.manager.subscribe(2)
    }
}

impl ::core::ops::Deref for super::LockedView<'_, key::Root_Network_Hostname> {
    type Target = ::heapless::String<32>;

    fn deref(&self) -> &Self::Target {
        &self.config.network_hostname
    }
}

#[allow(unused)]
impl super::View<key::Root_Network_Ap> {
    pub(crate) fn lock<T>(
        &self,
        f: impl FnOnce(super::LockedView<'_, key::Root_Network_Ap>) -> T,
    ) -> T {
        self.manager.state.lock(|state| {
            f(super::LockedView {
                config: &state.borrow().config,
                _key: ::core::marker::PhantomData,
            })
        })
    }

    pub(crate) fn password(&self) -> super::View<key::Root_Network_Ap_Password> {
        super::View {
            manager: self.manager,
            _key: ::core::marker::PhantomData,
        }
    }
}

#[allow(unused)]
impl super::LockedView<'_, key::Root_Network_Ap> {
    pub(crate) fn password(&self) -> super::LockedView<'_, key::Root_Network_Ap_Password> {
        super::LockedView {
            config: self.config,
            _key: ::core::marker::PhantomData,
        }
    }
}

#[allow(unused)]
impl super::View<key::Root_Network_Ap_Password> {
    pub(crate) fn lock<T>(&self, f: impl FnOnce(&::heapless::String<64>) -> T) -> T {
        self.manager
            .state
            .lock(|state| f(&state.borrow().config.network_ap_password))
    }

    pub(crate) fn subscribe(&self) -> Option<super::Subscriber> {
        self.manager.subscribe(3)
    }
}

impl ::core::ops::Deref for super::LockedView<'_, key::Root_Network_Ap_Password> {
    type Target = ::heapless::String<64>;

    fn deref(&self) -> &Self::Target {
        &self.config.network_ap_password
    }
}

#[allow(unused)]
impl super::View<key::Root_Network_Sta> {
    pub(crate) fn lock<T>(
        &self,
        f: impl FnOnce(super::LockedView<'_, key::Root_Network_Sta>) -> T,
    ) -> T {
        self.manager.state.lock(|state| {
            f(super::LockedView {
                config: &state.borrow().config,
                _key: ::core::marker::PhantomData,
            })
        })
    }

    pub(crate) fn ssid(&self) -> super::View<key::Root_Network_Sta_Ssid> {
        super::View {
            manager: self.manager,
            _key: ::core::marker::PhantomData,
        }
    }

    pub(crate) fn password(&self) -> super::View<key::Root_Network_Sta_Password> {
        super::View {
            manager: self.manager,
            _key: ::core::marker::PhantomData,
        }
    }
}

#[allow(unused)]
impl super::LockedView<'_, key::Root_Network_Sta> {
    pub(crate) fn ssid(&self) -> super::LockedView<'_, key::Root_Network_Sta_Ssid> {
        super::LockedView {
            config: self.config,
            _key: ::core::marker::PhantomData,
        }
    }

    pub(crate) fn password(&self) -> super::LockedView<'_, key::Root_Network_Sta_Password> {
        super::LockedView {
            config: self.config,
            _key: ::core::marker::PhantomData,
        }
    }
}

#[allow(unused)]
impl super::View<key::Root_Network_Sta_Ssid> {
    pub(crate) fn lock<T>(&self, f: impl FnOnce(&::heapless::String<32>) -> T) -> T {
        self.manager
            .state
            .lock(|state| f(&state.borrow().config.network_sta_ssid))
    }

    pub(crate) fn subscribe(&self) -> Option<super::Subscriber> {
        self.manager.subscribe(4)
    }
}

impl ::core::ops::Deref for super::LockedView<'_, key::Root_Network_Sta_Ssid> {
    type Target = ::heapless::String<32>;

    fn deref(&self) -> &Self::Target {
        &self.config.network_sta_ssid
    }
}

#[allow(unused)]
impl super::View<key::Root_Network_Sta_Password> {
    pub(crate) fn lock<T>(&self, f: impl FnOnce(&::heapless::String<64>) -> T) -> T {
        self.manager
            .state
            .lock(|state| f(&state.borrow().config.network_sta_password))
    }

    pub(crate) fn subscribe(&self) -> Option<super::Subscriber> {
        self.manager.subscribe(5)
    }
}

impl ::core::ops::Deref for super::LockedView<'_, key::Root_Network_Sta_Password> {
    type Target = ::heapless::String<64>;

    fn deref(&self) -> &Self::Target {
        &self.config.network_sta_password
    }
}

#[derive(Debug, Clone)]
pub(super) enum DeserializeError {
    WrongVersion,
    Postcard(postcard::Error),
}

impl RawConfig {
    pub(super) fn deserialize(from: &[u8]) -> Result<Self, DeserializeError> {
        let (version, from) = from.split_at(4);
        if version == u32::to_le_bytes(4) {
            postcard::from_bytes(from).map_err(DeserializeError::Postcard)
        } else {
            Err(DeserializeError::WrongVersion)
        }
    }

    pub(super) fn serialize(&self, buffer: &mut [u8]) -> postcard::Result<usize> {
        let (version, buffer) = buffer.split_at_mut(4);
        version.copy_from_slice(&u32::to_le_bytes(4));
        postcard::to_slice(self, buffer).map(|out| out.len() + 4)
    }

    pub(super) fn diff(&self, other: &Self, mut different: impl FnMut(usize)) {
        if self.name != other.name {
            different(0);
        }
        if self.leds_brightness != other.leds_brightness {
            different(1);
        }
        if self.network_hostname != other.network_hostname {
            different(2);
        }
        if self.network_ap_password != other.network_ap_password {
            different(3);
        }
        if self.network_sta_ssid != other.network_sta_ssid {
            different(4);
        }
        if self.network_sta_password != other.network_sta_password {
            different(5);
        }
    }
}
