#[derive(Debug, ::serde::Deserialize, ::serde::Serialize)]
#[allow(non_snake_case)]
pub(crate) struct RawConfig {
    pub(super) name: ::heapless::String<20>,
    pub(super) leds_brightness: u8,
    pub(super) display_brightness: u8,
    pub(super) network_hostname: ::heapless::String<32>,
    pub(super) network_password: ::heapless::String<64>,
    pub(super) network_home_ssid: ::heapless::String<32>,
    pub(super) network_home_password: ::heapless::String<64>,
    pub(super) expert: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for RawConfig {
    fn default() -> Self {
        Self {
            name: "VerTX".try_into().unwrap(),
            leds_brightness: 10,
            display_brightness: 255,
            network_hostname: "vertx".try_into().unwrap(),
            network_password: Default::default(),
            network_home_ssid: Default::default(),
            network_home_password: Default::default(),
            expert: false,
        }
    }
}
pub(crate) const BYTE_LENGTH: usize = 4 + 25 + 1 + 1 + 37 + 69 + 37 + 69 + 1;

#[derive(Debug, Clone)]
pub(super) enum DeserializeError {
    WrongVersion,
    Postcard(postcard::Error),
}

impl RawConfig {
    pub(super) fn deserialize(from: &[u8]) -> Result<Self, DeserializeError> {
        let (version, from) = from.split_at(4);
        if version == u32::to_le_bytes(1) {
            postcard::from_bytes(from).map_err(DeserializeError::Postcard)
        } else {
            Err(DeserializeError::WrongVersion)
        }
    }

    pub(super) fn serialize(&self, buffer: &mut [u8]) -> postcard::Result<usize> {
        let (version, buffer) = buffer.split_at_mut(4);
        version.copy_from_slice(&u32::to_le_bytes(1));
        postcard::to_slice(self, buffer).map(|out| out.len() + 4)
    }
}
