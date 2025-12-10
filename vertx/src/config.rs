#![cfg_attr(not(feature = "configurator"), expect(unused))]

mod codegen {
    include!(concat!(env!("OUT_DIR"), "/config.rs"));
}

use core::cell::RefCell;
use core::marker::PhantomData;
use core::{future, mem, task};

use embassy_sync::blocking_mutex::Mutex;
use static_cell::StaticCell;

use self::codegen::DeserializeError;
pub(crate) use self::codegen::{BYTE_LENGTH, RawConfig};

pub(crate) type RootConfig = View<codegen::key::Root>;

const SUBSCRIPTIONS: usize = 1;

#[derive(Clone, Copy)]
pub(crate) struct Manager {
    storage: crate::storage::Config,
    state: &'static Inner,
}

type Inner = Mutex<crate::mutex::SingleCore, RefCell<State>>;

struct State {
    modified: bool,
    config: RawConfig,
    subscriptions: heapless::Vec<(usize, Subscription), SUBSCRIPTIONS>,
}

impl Manager {
    pub(crate) async fn new(storage: crate::storage::Config) -> Self {
        static STATE: StaticCell<Inner> = StaticCell::new();

        let mut config = RawConfig::default();

        let mut buffer = [0; BYTE_LENGTH];
        let raw = storage.read(&mut buffer).await.unwrap();

        if raw.is_empty() {
            loog::debug!("No saved configuration");
        } else {
            loog::debug!("Loading configuration");

            match RawConfig::deserialize(raw) {
                Ok(loaded) => {
                    config = loaded;
                    loog::debug!("Successfully loaded configuration");
                }
                Err(DeserializeError::WrongVersion) => {
                    loog::error!("Invalid config version");
                }
                Err(DeserializeError::Postcard(err)) => {
                    loog::error!("Failed to load config: {err}");
                }
            }
        }

        Self {
            storage,
            state: STATE.init(Mutex::new(RefCell::new(State {
                modified: false,
                config,
                subscriptions: heapless::Vec::new(),
            }))),
        }
    }

    pub(crate) async fn replace(self, bytes: &[u8]) -> Result<(), ()> {
        let Ok(config) = RawConfig::deserialize(bytes) else {
            return Err(());
        };

        self.replace_impl(config).await;
        Ok(())
    }

    pub(crate) async fn reset(self) {
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

    pub(crate) async fn save(self) {
        loog::debug!("Writing configuration");

        let mut buffer = [0; BYTE_LENGTH];
        let mut len = 0;

        self.state.lock(|state| {
            let state = state.borrow();
            if !state.modified {
                return;
            }

            len = state.config.serialize(&mut buffer).unwrap();
        });

        if len > 0 {
            loog::unwrap!(self.storage.write(&buffer[0..len]).await);
        }
    }

    pub(crate) const fn config(self) -> RootConfig {
        View {
            manager: self,
            _key: PhantomData,
        }
    }

    /// Try to serialize the config
    pub(crate) fn serialize(self, buffer: &mut [u8]) -> postcard::Result<usize> {
        self.state
            .lock(|state| state.borrow().config.serialize(buffer))
    }

    pub(crate) fn subscribe(self, key: usize) -> Option<Subscriber> {
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
}

pub(crate) struct Subscriber {
    manager: Manager,
    subscription: usize,
}

impl Subscriber {
    pub(crate) fn updated(&self) -> impl Future<Output = ()> {
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
