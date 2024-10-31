use vertx_config::{minmax, storage, update};

#[allow(unused)]
#[derive(vertx_config::UpdateRef, vertx_config::Storage)]
pub struct Config {
    brightness: vertx_config::Reactive<minmax::U8<1, { u8::MAX }>, crate::mutex::SingleCore>,
    font_size: vertx_config::Reactive<FontSize, crate::mutex::SingleCore>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            brightness: minmax::U8::MAX.into(),
            font_size: Default::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum FontSize {
    Size7px = 7,
    #[default]
    Size9px = 9,
}

impl From<FontSize> for u8 {
    fn from(size: FontSize) -> Self {
        size as u8
    }
}

impl vertx_config::Storage for FontSize {
    async fn save<S: storage::Serializer>(&self, serializer: S) {
        serializer.integer(u8::from(*self).into());
    }

    fn load(from: storage::Stored<'_>) -> Self {
        use storage::Stored;

        match from {
            Stored::Integer(7) => Self::Size7px,
            Stored::Integer(9) => Self::Size9px,
            _ => Self::default(),
        }
    }
}

impl vertx_config::UpdateMut for FontSize {
    async fn update_mut<'a>(&mut self, key: &'a str, update: update::Update<'a>) -> update::Result {
        use update::{Error, Update};

        if !key.is_empty() {
            return Err(Error::KeyNotFound);
        }

        match update {
            Update::Integer(7) => *self = FontSize::Size7px,
            Update::Integer(9) => *self = FontSize::Size9px,
            Update::Integer(_) => return Err(Error::InvalidValue),
            _ => return Err(Error::InvalidType),
        }

        Ok(())
    }
}
