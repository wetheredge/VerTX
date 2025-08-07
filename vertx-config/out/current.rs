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
}
