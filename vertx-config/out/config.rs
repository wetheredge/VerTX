#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct RawConfig(
    // name
    ::heapless::String<20>,
    // leds.brightness
    u8,
    // display.brightness
    u8,
    // display.fontSize
    FontSize,
    // network.hostname
    ::heapless::String<32>,
    // network.password
    ::heapless::String<64>,
    // network.home.ssid
    ::heapless::String<32>,
    // network.home.password
    ::heapless::String<64>,
    // expert
    bool,
);

pub const BYTE_LENGTH: usize = 4 + 25 + 1 + 1 + 5 + 37 + 69 + 37 + 69 + 1;

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize)]
pub enum FontSize {
    /// 7px
    Size7px,
    /// 9px
    #[default]
    Size9px,
}

#[allow(non_camel_case_types, unused)]
mod key {
    #[derive(Clone, Copy)]
    pub(crate) struct Root;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Name;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Leds;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Leds_Brightness;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Display;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Display_Brightness;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Display_FontSize;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network_Hostname;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network_Password;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network_Home;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network_Home_Ssid;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Network_Home_Password;
    #[derive(Clone, Copy)]
    pub(crate) struct Root_Expert;
}

#[allow(unused)]
impl View<key::Root> {
    pub fn lock<T>(&self, f: impl FnOnce(LockedView<'_, key::Root>) -> T) -> T {
        self.manager.state.lock(|state| {
            f(LockedView {
                config: &state.borrow().config,
                _key: PhantomData,
            })
        })
    }

    pub fn name(&self) -> View<key::Root_Name> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }

    pub fn leds(&self) -> View<key::Root_Leds> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }

    pub fn display(&self) -> View<key::Root_Display> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }

    pub fn network(&self) -> View<key::Root_Network> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }

    pub fn expert(&self) -> View<key::Root_Expert> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }
}

#[allow(unused)]
impl LockedView<'_, key::Root> {
    pub fn name(&self) -> LockedView<'_, key::Root_Name> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }

    pub fn leds(&self) -> LockedView<'_, key::Root_Leds> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }

    pub fn display(&self) -> LockedView<'_, key::Root_Display> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }

    pub fn network(&self) -> LockedView<'_, key::Root_Network> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }

    pub fn expert(&self) -> LockedView<'_, key::Root_Expert> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }
}

#[allow(unused)]
impl View<key::Root_Name> {
    pub fn lock<T>(&self, f: impl FnOnce(&::heapless::String<20>) -> T) -> T {
        self.manager.state.lock(|state| f(&state.borrow().config.0))
    }

    pub fn subscribe(&self) -> Option<Subscriber> {
        self.manager.subscribe(0)
    }
}

impl ::core::ops::Deref for LockedView<'_, key::Root_Name> {
    type Target = ::heapless::String<20>;

    fn deref(&self) -> &Self::Target {
        &self.config.0
    }
}

#[allow(unused)]
impl View<key::Root_Leds> {
    pub fn lock<T>(&self, f: impl FnOnce(LockedView<'_, key::Root_Leds>) -> T) -> T {
        self.manager.state.lock(|state| {
            f(LockedView {
                config: &state.borrow().config,
                _key: PhantomData,
            })
        })
    }

    pub fn brightness(&self) -> View<key::Root_Leds_Brightness> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }
}

#[allow(unused)]
impl LockedView<'_, key::Root_Leds> {
    pub fn brightness(&self) -> LockedView<'_, key::Root_Leds_Brightness> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }
}

#[allow(unused)]
impl View<key::Root_Leds_Brightness> {
    pub fn lock<T>(&self, f: impl FnOnce(&u8) -> T) -> T {
        self.manager.state.lock(|state| f(&state.borrow().config.1))
    }

    pub fn subscribe(&self) -> Option<Subscriber> {
        self.manager.subscribe(1)
    }
}

impl ::core::ops::Deref for LockedView<'_, key::Root_Leds_Brightness> {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.config.1
    }
}

#[allow(unused)]
impl View<key::Root_Display> {
    pub fn lock<T>(&self, f: impl FnOnce(LockedView<'_, key::Root_Display>) -> T) -> T {
        self.manager.state.lock(|state| {
            f(LockedView {
                config: &state.borrow().config,
                _key: PhantomData,
            })
        })
    }

    pub fn brightness(&self) -> View<key::Root_Display_Brightness> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }

    pub fn font_size(&self) -> View<key::Root_Display_FontSize> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }
}

#[allow(unused)]
impl LockedView<'_, key::Root_Display> {
    pub fn brightness(&self) -> LockedView<'_, key::Root_Display_Brightness> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }

    pub fn font_size(&self) -> LockedView<'_, key::Root_Display_FontSize> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }
}

#[allow(unused)]
impl View<key::Root_Display_Brightness> {
    pub fn lock<T>(&self, f: impl FnOnce(&u8) -> T) -> T {
        self.manager.state.lock(|state| f(&state.borrow().config.2))
    }

    pub fn subscribe(&self) -> Option<Subscriber> {
        self.manager.subscribe(2)
    }
}

impl ::core::ops::Deref for LockedView<'_, key::Root_Display_Brightness> {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.config.2
    }
}

#[allow(unused)]
impl View<key::Root_Display_FontSize> {
    pub fn lock<T>(&self, f: impl FnOnce(&FontSize) -> T) -> T {
        self.manager.state.lock(|state| f(&state.borrow().config.3))
    }

    pub fn subscribe(&self) -> Option<Subscriber> {
        self.manager.subscribe(3)
    }
}

impl ::core::ops::Deref for LockedView<'_, key::Root_Display_FontSize> {
    type Target = FontSize;

    fn deref(&self) -> &Self::Target {
        &self.config.3
    }
}

#[allow(unused)]
impl View<key::Root_Network> {
    pub fn lock<T>(&self, f: impl FnOnce(LockedView<'_, key::Root_Network>) -> T) -> T {
        self.manager.state.lock(|state| {
            f(LockedView {
                config: &state.borrow().config,
                _key: PhantomData,
            })
        })
    }

    pub fn hostname(&self) -> View<key::Root_Network_Hostname> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }

    pub fn password(&self) -> View<key::Root_Network_Password> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }

    pub fn home(&self) -> View<key::Root_Network_Home> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }
}

#[allow(unused)]
impl LockedView<'_, key::Root_Network> {
    pub fn hostname(&self) -> LockedView<'_, key::Root_Network_Hostname> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }

    pub fn password(&self) -> LockedView<'_, key::Root_Network_Password> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }

    pub fn home(&self) -> LockedView<'_, key::Root_Network_Home> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }
}

#[allow(unused)]
impl View<key::Root_Network_Hostname> {
    pub fn lock<T>(&self, f: impl FnOnce(&::heapless::String<32>) -> T) -> T {
        self.manager.state.lock(|state| f(&state.borrow().config.4))
    }

    pub fn subscribe(&self) -> Option<Subscriber> {
        self.manager.subscribe(4)
    }
}

impl ::core::ops::Deref for LockedView<'_, key::Root_Network_Hostname> {
    type Target = ::heapless::String<32>;

    fn deref(&self) -> &Self::Target {
        &self.config.4
    }
}

#[allow(unused)]
impl View<key::Root_Network_Password> {
    pub fn lock<T>(&self, f: impl FnOnce(&::heapless::String<64>) -> T) -> T {
        self.manager.state.lock(|state| f(&state.borrow().config.5))
    }

    pub fn subscribe(&self) -> Option<Subscriber> {
        self.manager.subscribe(5)
    }
}

impl ::core::ops::Deref for LockedView<'_, key::Root_Network_Password> {
    type Target = ::heapless::String<64>;

    fn deref(&self) -> &Self::Target {
        &self.config.5
    }
}

#[allow(unused)]
impl View<key::Root_Network_Home> {
    pub fn lock<T>(&self, f: impl FnOnce(LockedView<'_, key::Root_Network_Home>) -> T) -> T {
        self.manager.state.lock(|state| {
            f(LockedView {
                config: &state.borrow().config,
                _key: PhantomData,
            })
        })
    }

    pub fn ssid(&self) -> View<key::Root_Network_Home_Ssid> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }

    pub fn password(&self) -> View<key::Root_Network_Home_Password> {
        View {
            manager: self.manager,
            _key: PhantomData,
        }
    }
}

#[allow(unused)]
impl LockedView<'_, key::Root_Network_Home> {
    pub fn ssid(&self) -> LockedView<'_, key::Root_Network_Home_Ssid> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }

    pub fn password(&self) -> LockedView<'_, key::Root_Network_Home_Password> {
        LockedView {
            config: self.config,
            _key: PhantomData,
        }
    }
}

#[allow(unused)]
impl View<key::Root_Network_Home_Ssid> {
    pub fn lock<T>(&self, f: impl FnOnce(&::heapless::String<32>) -> T) -> T {
        self.manager.state.lock(|state| f(&state.borrow().config.6))
    }

    pub fn subscribe(&self) -> Option<Subscriber> {
        self.manager.subscribe(6)
    }
}

impl ::core::ops::Deref for LockedView<'_, key::Root_Network_Home_Ssid> {
    type Target = ::heapless::String<32>;

    fn deref(&self) -> &Self::Target {
        &self.config.6
    }
}

#[allow(unused)]
impl View<key::Root_Network_Home_Password> {
    pub fn lock<T>(&self, f: impl FnOnce(&::heapless::String<64>) -> T) -> T {
        self.manager.state.lock(|state| f(&state.borrow().config.7))
    }

    pub fn subscribe(&self) -> Option<Subscriber> {
        self.manager.subscribe(7)
    }
}

impl ::core::ops::Deref for LockedView<'_, key::Root_Network_Home_Password> {
    type Target = ::heapless::String<64>;

    fn deref(&self) -> &Self::Target {
        &self.config.7
    }
}

#[allow(unused)]
impl View<key::Root_Expert> {
    pub fn lock<T>(&self, f: impl FnOnce(&bool) -> T) -> T {
        self.manager.state.lock(|state| f(&state.borrow().config.8))
    }

    pub fn subscribe(&self) -> Option<Subscriber> {
        self.manager.subscribe(8)
    }
}

impl ::core::ops::Deref for LockedView<'_, key::Root_Expert> {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.config.8
    }
}

#[derive(Debug, Clone, ::serde::Deserialize)]
#[allow(non_camel_case_types)]
#[serde(tag = "key", content = "value")]
pub enum Update<'a> {
    #[serde(borrow)]
    Root_Name(&'a str),
    Root_Leds_Brightness(u8),
    Root_Display_Brightness(u8),
    Root_Display_FontSize(FontSize),
    #[serde(borrow)]
    Root_Network_Hostname(&'a str),
    #[serde(borrow)]
    Root_Network_Password(&'a str),
    #[serde(borrow)]
    Root_Network_Home_Ssid(&'a str),
    #[serde(borrow)]
    Root_Network_Home_Password(&'a str),
    Root_Expert(bool),
}

impl RawConfig {
    fn deserialize(from: &[u8]) -> Result<Self, LoadError> {
        let (version, from) = from.split_at(4);
        if version == u32::to_le_bytes(0) {
            postcard::from_bytes(from).map_err(LoadError::Postcard)
        } else {
            Err(LoadError::WrongVersion)
        }
    }

    fn serialize(&self, buffer: &mut [u8]) -> postcard::Result<usize> {
        let (version, buffer) = buffer.split_at_mut(4);
        version.copy_from_slice(&u32::to_le_bytes(0));
        postcard::to_slice(self, buffer).map(|out| out.len() + 4)
    }

    fn update(&mut self, update: Update<'_>) -> Result<usize, UpdateError> {
        match update {
            Update::Root_Name(update) => {
                let Ok(update) = update.try_into() else {
                    return Err(UpdateError::TooLarge { max: 20 });
                };
                self.0 = update;
                Ok(0)
            }
            Update::Root_Leds_Brightness(update) => {
                if update < 10 {
                    return Err(UpdateError::TooSmall { min: 10 });
                };
                self.1 = update;
                Ok(1)
            }
            Update::Root_Display_Brightness(update) => {
                if update < 1 {
                    return Err(UpdateError::TooSmall { min: 1 });
                };
                self.2 = update;
                Ok(2)
            }
            Update::Root_Display_FontSize(update) => {
                self.3 = update;
                Ok(3)
            }
            Update::Root_Network_Hostname(update) => {
                let Ok(update) = update.try_into() else {
                    return Err(UpdateError::TooLarge { max: 32 });
                };
                self.4 = update;
                Ok(4)
            }
            Update::Root_Network_Password(update) => {
                let Ok(update) = update.try_into() else {
                    return Err(UpdateError::TooLarge { max: 64 });
                };
                self.5 = update;
                Ok(5)
            }
            Update::Root_Network_Home_Ssid(update) => {
                let Ok(update) = update.try_into() else {
                    return Err(UpdateError::TooLarge { max: 32 });
                };
                self.6 = update;
                Ok(6)
            }
            Update::Root_Network_Home_Password(update) => {
                let Ok(update) = update.try_into() else {
                    return Err(UpdateError::TooLarge { max: 64 });
                };
                self.7 = update;
                Ok(7)
            }
            Update::Root_Expert(update) => {
                self.8 = update;
                Ok(8)
            }
        }
    }
}
