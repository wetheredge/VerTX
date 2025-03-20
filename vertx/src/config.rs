#![cfg_attr(not(feature = "configurator"), expect(unused))]

mod codegen {
    include!(concat!(env!("OUT_DIR"), "/config.rs"));
}

use core::cell::RefCell;
use core::marker::PhantomData;
use core::{future, mem, task};

use embassy_sync::blocking_mutex::Mutex;
use portable_atomic::AtomicBool;
use serde::{Deserialize, Serialize};
use static_cell::StaticCell;

use self::codegen::DeserializeError;
pub(crate) use self::codegen::{BYTE_LENGTH, RawConfig};
use crate::hal::prelude::*;
use crate::storage::File;

pub(crate) type RootConfig = View<codegen::key::Root>;

const SUBSCRIPTIONS: usize = 1;

#[derive(Clone, Copy)]
pub struct Manager {
    init: &'static AtomicBool,
    state: &'static Mutex<crate::mutex::SingleCore, RefCell<State>>,
}

struct State {
    modified: bool,
    config: RawConfig,
    file: Option<File>,
    subscriptions: heapless::Vec<(usize, Subscription), SUBSCRIPTIONS>,
}

impl Manager {
    pub fn new() -> Self {
        static INIT: AtomicBool = AtomicBool::new(false);
        static STATE: StaticCell<Mutex<crate::mutex::SingleCore, RefCell<State>>> =
            StaticCell::new();

        Self {
            init: &INIT,
            state: STATE.init(Mutex::new(RefCell::new(State {
                modified: false,
                config: RawConfig::default(),
                file: None,
                subscriptions: heapless::Vec::new(),
            }))),
        }
    }

    pub async fn load(self, mut file: File) {
        let mut buffer = [0; BYTE_LENGTH];
        let mut len = 0;
        loop {
            let chunk = loog::unwrap!(file.read(&mut buffer[len..]).await);
            if chunk == 0 {
                break;
            }
            len += chunk;
        }

        self.state.lock(|state| {
            let mut state = state.borrow_mut();
            if state.file.replace(file).is_some() {
                loog::warn!("Called config::Manager::load() more than once");
            }
        });

        if len == 0 {
            loog::debug!("No saved configuration");
        } else {
            loog::debug!("Loading configuration");

            let raw = match RawConfig::deserialize(&buffer[..len]) {
                Ok(raw) => raw,
                Err(DeserializeError::WrongVersion) => {
                    loog::error!("Invalid config version");
                    return;
                }
                Err(DeserializeError::Postcard(err)) => {
                    loog::error!("Failed to load config: {err}");
                    return;
                }
            };

            self.replace_impl(raw).await;
            loog::debug!("Finished loading configuration");
        }

        self.init.store(true, portable_atomic::Ordering::Relaxed);
    }

    pub async fn replace(self, bytes: &[u8]) -> Result<(), ()> {
        let Ok(config) = RawConfig::deserialize(bytes) else {
            return Err(());
        };

        self.replace_impl(config).await;
        Ok(())
    }

    pub async fn reset(self) {
        self.replace_impl(RawConfig::default()).await;
    }

    async fn replace_impl(self, mut config: RawConfig) {
        self.state.lock(|state| {
            let mut state = state.borrow_mut();
            let state = &mut *state;

            mem::swap(&mut state.config, &mut config);

            let mut modified = false;
            state.config.diff(&config, |key| {
                modified = true;
                for (sub_key, sub) in &mut state.subscriptions {
                    if *sub_key == key {
                        let old_sub = mem::replace(sub, Subscription::Updated);
                        if let Subscription::Waiting(w) = old_sub {
                            w.wake();
                        }
                    }
                }
            });
            state.modified |= modified;
        });
    }

    pub async fn save(self) {
        self.wait_for_init().await;

        loog::debug!("Writing configuration");
        let mut buffer = [0; BYTE_LENGTH];

        let to_write = self.state.lock(|state| {
            let state = state.borrow();
            if !state.modified {
                return None;
            }

            // Should be Some after self.wait_for_init() completes
            let file = state.file.clone().unwrap();

            let len = state.config.serialize(&mut buffer).unwrap();
            Some((file, &buffer[0..len]))
        });

        if let Some((mut file, data)) = to_write {
            loog::unwrap!(file.truncate().await);
            loog::unwrap!(file.write_all(data).await);
            loog::unwrap!(file.flush().await);
        }
    }

    pub const fn config(self) -> RootConfig {
        View {
            manager: self,
            _key: PhantomData,
        }
    }

    /// Try to serialize the config. If it has not been initialized yet, this
    /// returns `None`.
    pub fn serialize(self, buffer: &mut [u8]) -> Option<postcard::Result<usize>> {
        self.is_initted().then(|| {
            self.state
                .lock(|state| state.borrow().config.serialize(buffer))
        })
    }

    pub fn subscribe(self, key: usize) -> Option<Subscriber> {
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

    fn poll(self, id: usize, ctx: &mut task::Context<'_>) -> task::Poll<()> {
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

    fn is_initted(self) -> bool {
        self.init.load(portable_atomic::Ordering::Relaxed)
    }

    /// Busy-wait until initialized
    async fn wait_for_init(self) {
        while !self.is_initted() {
            embassy_futures::yield_now().await;
        }
    }
}

pub(crate) struct Subscriber {
    manager: Manager,
    subscription: usize,
}

impl Subscriber {
    pub fn updated(&self) -> impl Future<Output = ()> {
        future::poll_fn(move |ctx| self.manager.poll(self.subscription, ctx))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct View<K> {
    manager: Manager,
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
