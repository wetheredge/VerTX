use core::fmt;

use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::mutex::{Mutex, MutexGuard};

use crate::storage::{Serializer, Storage, Stored};
use crate::update::{self, Update, UpdateMut, UpdateRef};

pub struct BootSnapshot<T, M: RawMutex> {
    current: Mutex<M, T>,
    boot: T,
}

impl<T, M: RawMutex> BootSnapshot<T, M> {
    pub fn boot(&self) -> &T {
        &self.boot
    }

    pub async fn current(&self) -> MutexGuard<'_, M, T> {
        self.current.lock().await
    }
}

impl<T: fmt::Debug, M: RawMutex> fmt::Debug for BootSnapshot<T, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BootSnapshot")
            .field("boot", &self.boot)
            .finish_non_exhaustive()
    }
}

impl<T: Clone, M: RawMutex> From<T> for BootSnapshot<T, M> {
    fn from(value: T) -> Self {
        Self {
            current: Mutex::new(value.clone()),
            boot: value,
        }
    }
}

impl<T: Default, M: RawMutex> Default for BootSnapshot<T, M> {
    fn default() -> Self {
        Self {
            current: Mutex::new(T::default()),
            boot: T::default(),
        }
    }
}

impl<T: Storage + Clone, M: RawMutex> Storage for BootSnapshot<T, M> {
    async fn save<S: Serializer>(&self, serializer: S) {
        let value = self.current().await;
        value.save(serializer).await;
    }

    fn load(from: Stored<'_>) -> Self {
        T::load(from).into()
    }
}

impl<T: Clone + UpdateMut, M: RawMutex> UpdateRef for BootSnapshot<T, M> {
    async fn update_ref<'a>(&self, key: &'a str, update: Update<'a>) -> update::Result {
        let mut current = self.current.lock().await;
        current.update_mut(key, update).await
    }
}
