use core::fmt;

use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::mutex::{Mutex, MutexGuard};
use embassy_sync::pubsub::{self, PubSubChannel};

use crate::storage::{Serializer, Storage, Stored};
use crate::update::{self, Update, UpdateMut, UpdateRef};

pub struct Reactive<T: ?Sized, M: RawMutex, const N: usize = 1> {
    updated: PubSubChannel<M, (), 1, N, 0>,
    current: Mutex<M, T>,
}

pub struct ReactiveSubscriber<'a, M: RawMutex, const N: usize>(
    pubsub::Subscriber<'a, M, (), 1, N, 0>,
);

impl<T: ?Sized, M: RawMutex, const N: usize> Reactive<T, M, N> {
    pub async fn current(&self) -> MutexGuard<'_, M, T> {
        self.current.lock().await
    }

    pub fn subscriber(&self) -> Option<ReactiveSubscriber<'_, M, N>> {
        self.updated.subscriber().ok().map(ReactiveSubscriber)
    }
}

impl<M: RawMutex, const N: usize> ReactiveSubscriber<'_, M, N> {
    pub async fn wait(&mut self) {
        self.0.next_message_pure().await;
    }
}

impl<T: ?Sized, M: RawMutex, const N: usize> fmt::Debug for Reactive<T, M, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Reactive").finish_non_exhaustive()
    }
}

impl<M: RawMutex, const N: usize> fmt::Debug for ReactiveSubscriber<'_, M, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReactiveSubscriber").finish_non_exhaustive()
    }
}

impl<T, M: RawMutex> From<T> for Reactive<T, M> {
    fn from(value: T) -> Self {
        Self {
            updated: PubSubChannel::new(),
            current: Mutex::new(value),
        }
    }
}

impl<T: ?Sized + Default, M: RawMutex, const N: usize> Default for Reactive<T, M, N> {
    fn default() -> Self {
        Self {
            updated: PubSubChannel::new(),
            current: Mutex::new(T::default()),
        }
    }
}

impl<T, M> Storage for Reactive<T, M>
where
    T: ?Sized + Storage + Clone,
    M: RawMutex,
{
    async fn save<S: Serializer>(&self, serializer: S) {
        let value = self.current().await;
        value.save(serializer).await;
    }

    fn load(from: Stored<'_>) -> Self {
        T::load(from).into()
    }
}

impl<T, M> UpdateRef for Reactive<T, M>
where
    T: ?Sized + Clone + UpdateMut,
    M: RawMutex,
{
    async fn update_ref<'a>(&self, key: &'a str, update: Update<'a>) -> update::Result {
        let mut current = self.current.lock().await;
        current.update_mut(key, update).await?;
        self.updated.immediate_publisher().publish_immediate(());
        Ok(())
    }
}
