#[derive(Debug, Default, ::serde::Deserialize, ::serde::Serialize)]
#[allow(non_snake_case)]
pub(crate) struct RawConfig {
    pub(super) name: ::heapless::String<20>,
    pub(super) leds_brightness: u8,
    pub(super) display_brightness: u8,
    pub(super) display_fontSize: FontSize,
    pub(super) network_hostname: ::heapless::String<32>,
    pub(super) network_password: ::heapless::String<64>,
    pub(super) network_home_ssid: ::heapless::String<32>,
    pub(super) network_home_password: ::heapless::String<64>,
    pub(super) expert: bool,
}

pub(crate) const BYTE_LENGTH: usize = 4 + 25 + 1 + 1 + 5 + 37 + 69 + 37 + 69 + 1;

#[derive(Debug, Default, Clone, Copy, ::serde::Deserialize, ::serde::Serialize)]
pub(crate) enum FontSize {
    /// 7px
    Size7px,
    /// 9px
    #[default]
    Size9px,
}

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
