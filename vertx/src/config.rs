#![cfg_attr(not(feature = "configurator"), expect(unused))]

mod codegen {
    include!(concat!(env!("OUT_DIR"), "/config.rs"));
}

use core::cell::RefCell;
use core::marker::PhantomData;
use core::{future, mem, task};

use embassy_sync::blocking_mutex::Mutex;
use serde::{Deserialize, Serialize};

use self::codegen::DeserializeError;
pub(crate) use self::codegen::{BYTE_LENGTH, RawConfig, Update};
use crate::hal::prelude::*;

pub(crate) type RootConfig = View<codegen::key::Root>;

const SUBSCRIPTIONS: usize = 1;

struct ManagerState {
    modified: bool,
    config: RawConfig,
    storage: crate::hal::ConfigStorage,
    subscriptions: heapless::Vec<(usize, Subscription), SUBSCRIPTIONS>,
}

pub struct Manager {
    state: Mutex<crate::mutex::SingleCore, RefCell<ManagerState>>,
}

impl Manager {
    pub fn load(storage: crate::hal::ConfigStorage) -> Self {
        let raw = storage
            .load(|bytes| {
                match RawConfig::deserialize(bytes) {
                    Err(DeserializeError::WrongVersion) => loog::error!("Invalid config version"),
                    Err(DeserializeError::Postcard(err)) => {
                        loog::error!("Failed to load config: {err}");
                    }
                    Ok(raw) => return Some(raw),
                }

                None
            })
            .unwrap_or_default();

        Self {
            state: Mutex::new(RefCell::new(ManagerState {
                modified: false,
                config: raw,
                storage,
                subscriptions: heapless::Vec::new(),
            })),
        }
    }

    pub async fn update(&self, update: Update<'_>) -> Result<(), UpdateError> {
        self.state.lock(|state| {
            let mut state = state.borrow_mut();
            state.modified = true;
            state.config.update(update).map(|key| {
                for (sub_key, sub) in &mut state.subscriptions {
                    if *sub_key == key {
                        let old_sub = mem::replace(sub, Subscription::Updated);
                        if let Subscription::Waiting(w) = old_sub {
                            w.wake();
                        }
                    }
                }
            })
        })
    }

    pub async fn save(&self) {
        self.state.lock(|state| {
            let mut state = state.borrow_mut();
            if !state.modified {
                return;
            }

            loog::info!("Writing configuration");
            let mut buffer = [0; BYTE_LENGTH];
            let len = state.config.serialize(&mut buffer).unwrap();
            state.storage.save(&buffer[0..len]);
        });
    }

    pub const fn config(&'static self) -> RootConfig {
        View {
            manager: self,
            _key: PhantomData,
        }
    }

    pub fn serialize(&self, buffer: &mut [u8]) -> postcard::Result<usize> {
        self.state
            .lock(|state| state.borrow().config.serialize(buffer))
    }

    pub fn subscribe(&'static self, key: usize) -> Option<Subscriber> {
        self.state.lock(|state| {
            let mut state = state.borrow_mut();
            (state.subscriptions.len() < SUBSCRIPTIONS).then(|| {
                state.subscriptions.push((key, Subscription::None)).unwrap();
                Subscriber {
                    manager: self,
                    subscription: state.subscriptions.len() - 1,
                }
            })
        })
    }

    fn poll(&self, id: usize, ctx: &mut task::Context<'_>) -> task::Poll<()> {
        self.state.lock(|state| {
            let mut state = state.borrow_mut();
            let sub = state.subscriptions.get_mut(id).unwrap();
            match sub.1.clone() {
                Subscription::None => {
                    sub.1 = Subscription::Waiting(ctx.waker().clone());
                    task::Poll::Pending
                }
                Subscription::Waiting(w) => {
                    if !w.will_wake(ctx.waker()) {
                        sub.1 = Subscription::Waiting(ctx.waker().clone());
                        w.wake();
                    }
                    task::Poll::Pending
                }
                Subscription::Updated => {
                    sub.1 = Subscription::None;
                    task::Poll::Ready(())
                }
            }
        })
    }
}

pub(crate) struct Subscriber {
    manager: &'static Manager,
    subscription: usize,
}

impl Subscriber {
    pub fn updated(&self) -> impl Future<Output = ()> {
        future::poll_fn(move |ctx| self.manager.poll(self.subscription, ctx))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct View<K> {
    manager: &'static Manager,
    _key: PhantomData<K>,
}

pub(crate) struct LockedView<'a, K> {
    config: &'a RawConfig,
    _key: PhantomData<K>,
}

#[derive(Debug, Clone)]
enum Subscription {
    None,
    Waiting(task::Waker),
    Updated,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum UpdateError {
    TooLarge { max: i64 },
    TooSmall { min: i64 },
}
