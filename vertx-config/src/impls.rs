use alloc::borrow::ToOwned;

use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::mutex::Mutex;

use crate::storage::{Serializer, Storage, Stored};
use crate::update::{self, Update, UpdateMut, UpdateRef};

macro_rules! impl_config {
    ($kind:ident($t:ty), $serialize:ident) => {
        impl Storage for $t {
            async fn save<S: Serializer>(&self, serializer: S) {
                serializer.$serialize(*self);
            }

            fn load(from: Stored<'_>) -> Self {
                if let Stored::$kind(from) = from {
                    from
                } else {
                    Self::default()
                }
            }
        }

        impl UpdateMut for $t {
            async fn update_mut<'a>(&mut self, key: &'a str, update: Update<'a>) -> update::Result {
                if !key.is_empty() {
                    return Err(update::Error::KeyNotFound);
                }

                let Update::$kind(update) = update else {
                    return Err(update::Error::InvalidType);
                };

                *self = update;
                Ok(())
            }
        }
    };
    (Integer($($t:ty),+)) => {$(
        impl Storage for $t {
            async fn save<S: Serializer>(&self, serializer: S) {
                serializer.integer((*self).into());
            }

            fn load(from: Stored<'_>) -> Self {
                if let Stored::Integer(from) = from {
                    if from >= Self::MIN.into() && from <= Self::MAX.into() {
                        return from as $t;
                    }
                }

                Self::default()
            }
        }

        impl UpdateMut for $t {
            async fn update_mut<'a>(&mut self, key: &'a str, update: Update<'a>) -> update::Result {
                if !key.is_empty() {
                    return Err(update::Error::KeyNotFound);
                }

                let Update::Integer(update) = update else {
                    return Err(update::Error::InvalidType);
                };

                if update < Self::MIN.into() {
                    return Err(update::Error::TooSmall { min: Self::MIN.into() });
                }

                if update > Self::MAX.into() {
                    return Err(update::Error::TooLarge { max: Self::MAX.into() });
                }

                *self = update as $t;
                Ok(())
            }
        }
    )+};
}

impl_config!(Boolean(bool), boolean);
impl_config!(Float(f32), float);
impl_config!(Integer(u8, i8, u16, i16, u32, i32));

impl Storage for alloc::string::String {
    async fn save<S: Serializer>(&self, serializer: S) {
        serializer.string(self);
    }

    fn load(from: Stored<'_>) -> Self {
        if let Stored::String(str) = from {
            str.to_owned()
        } else {
            Self::new()
        }
    }
}

impl UpdateMut for alloc::string::String {
    async fn update_mut<'a>(&mut self, key: &'a str, update: Update<'a>) -> update::Result {
        if !key.is_empty() {
            return Err(update::Error::KeyNotFound);
        }

        let Update::String(value) = update else {
            return Err(update::Error::InvalidType);
        };

        self.clear();
        self.push_str(value);

        Ok(())
    }
}

impl<T: Storage, M: RawMutex> Storage for Mutex<M, T> {
    async fn save<S: Serializer>(&self, serializer: S) {
        let inner = self.lock().await;
        inner.save(serializer).await;
    }

    fn load(from: Stored<'_>) -> Self {
        Self::new(T::load(from))
    }
}

impl<T: UpdateMut, M: RawMutex> UpdateRef for Mutex<M, T> {
    async fn update_ref<'a>(&self, key: &'a str, update: Update<'a>) -> update::Result {
        let mut inner = self.lock().await;
        inner.update_mut(key, update).await
    }
}

#[cfg(feature = "heapless")]
mod heapless {
    use super::*;

    impl<const N: usize> crate::Storage for ::heapless::String<N> {
        async fn save<S: crate::storage::Serializer>(&self, serializer: S) {
            serializer.string(self);
        }

        fn load(from: Stored<'_>) -> Self {
            if let Stored::String(from) = from {
                if let Ok(str) = from.try_into() {
                    return str;
                }
            }

            Self::new()
        }
    }

    impl<const N: usize> crate::UpdateMut for ::heapless::String<N> {
        async fn update_mut<'a>(&mut self, key: &'a str, update: Update<'a>) -> update::Result {
            if !key.is_empty() {
                return Err(update::Error::KeyNotFound);
            }

            let Update::String(value) = update else {
                return Err(update::Error::InvalidType);
            };

            if value.len() > N {
                return Err(update::Error::TooLarge { max: N as i64 });
            }

            self.clear();
            // Will always succeed after above check
            let _ = self.push_str(value);

            Ok(())
        }
    }
}
