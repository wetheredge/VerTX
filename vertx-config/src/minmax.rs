use core::ops;

use crate::storage::{self, Storage};
use crate::update::{self, UpdateMut};

macro_rules! def {
    ($name:ident, $native:ty) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name<const MIN: $native, const MAX: $native> {
            inner: $native,
        }

        impl<const MIN: $native, const MAX: $native> $name<MIN, MAX> {
            pub const MAX: Self = Self { inner: MAX };
            pub const MIN: Self = Self { inner: MIN };
        }

        impl<const MIN: $native, const MAX: $native> ops::Deref for $name<MIN, MAX> {
            type Target = $native;

            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl<const MIN: $native, const MAX: $native> Storage for $name<MIN, MAX> {
            async fn save<S: storage::Serializer>(&self, serializer: S) {
                serializer.integer(self.inner.into());
            }

            fn load(from: storage::Stored<'_>) -> Self {
                Self {
                    inner: <$native>::load(from).max(MIN).min(MAX),
                }
            }
        }

        impl<const MIN: $native, const MAX: $native> UpdateMut for $name<MIN, MAX> {
            async fn update_mut<'a>(
                &mut self,
                key: &'a str,
                update: update::Update<'a>,
            ) -> update::Result {
                if !key.is_empty() {
                    return Err(update::Error::KeyNotFound);
                }

                let update::Update::Integer(update) = update else {
                    return Err(update::Error::InvalidType);
                };

                if update < MIN.into() {
                    return Err(update::Error::TooSmall { min: MIN.into() });
                }

                if update > MAX.into() {
                    return Err(update::Error::TooLarge { max: MAX.into() });
                }

                self.inner = update as $native;
                Ok(())
            }
        }
    };
}

def!(U8, u8);
def!(I8, i8);
def!(U16, u16);
def!(I16, i16);
def!(U32, u32);
def!(I32, i32);
