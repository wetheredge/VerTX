use alloc::borrow::Cow;
use alloc::vec::Vec;

use embassy_executor::{task, Spawner};
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};
use static_cell::make_static;
use vertx_network::api::{Body, Method, Response};

pub(crate) type StatusSignal = Signal<crate::mutex::SingleCore, Status>;

#[derive(Debug, Clone, Copy)]
#[allow(unused)]
pub(crate) struct Status {
    /// Per cell battery voltage in centivolts
    pub(crate) battery_voltage: u16,
    pub(crate) idle_time: f32,
    pub(crate) timing_drift: f32,
}

#[derive(Debug, serde::Serialize)]
enum ConfigUpdateResult {
    Ok,
    TooLarge { max: i64 },
    TooSmall { min: i64 },
}

impl From<Result<(), crate::config::UpdateError>> for ConfigUpdateResult {
    fn from(value: Result<(), crate::config::UpdateError>) -> Self {
        use crate::config::UpdateError;

        match value {
            Ok(()) => Self::Ok,
            Err(UpdateError::TooLarge { max }) => Self::TooLarge { max },
            Err(UpdateError::TooSmall { min }) => Self::TooSmall { min },
        }
    }
}

pub(crate) struct Api {
    reset: &'static crate::reset::Manager,
    config: &'static crate::config::Manager,
    #[allow(unused)]
    status: &'static StatusSignal,
}

impl Api {
    pub(crate) fn new(
        spawner: Spawner,
        reset: &'static crate::reset::Manager,
        config: &'static crate::config::Manager,
    ) -> Self {
        let status_signal = make_static!(StatusSignal::new());
        spawner.must_spawn(status(status_signal));

        Self {
            reset,
            config,
            status: status_signal,
        }
    }
}

impl vertx_network::Api for Api {
    async fn handle(&self, path: &str, method: Method, request: &[u8]) -> Response {
        let path = path.strip_suffix('/').unwrap_or(path);

        // TODO
        // loog::debug!("Received api request: {method} {path}");

        macro_rules! is_method {
            ($method:ident) => {
                if method != Method::$method {
                    return Response::MethodNotAllowed(Cow::Borrowed(&[Method::$method]));
                }
            };
        }

        match path {
            "build-info" => {
                is_method!(Get);
                let body = include_bytes!(concat!(env!("OUT_DIR"), "/build_info.json"));
                Response::Ok(Some(Body {
                    mime: b"application/json".into(),
                    body: body.into(),
                }))
            }
            "shutdown" => {
                is_method!(Post);
                self.reset.shut_down();
                Response::Ok(None)
            }
            "reboot" => {
                is_method!(Post);
                self.reset.reboot();
                Response::Ok(None)
            }
            "exit" => {
                is_method!(Post);
                self.reset.reboot_into(crate::BootMode::Standard).await;
                Response::Ok(None)
            }
            "config" => match method {
                Method::Get => {
                    let mut config =
                        bytemuck::allocation::zeroed_slice_box(crate::config::BYTE_LENGTH);
                    self.config.serialize(&mut config).unwrap();

                    let config = Vec::from(config);
                    Response::Ok(Some(Body {
                        mime: b"application/octet-stream".into(),
                        body: config.into(),
                    }))
                }
                Method::Patch => {
                    let Ok((update, _)) = serde_json_core::from_slice(request) else {
                        return Response::BadRequest {
                            reason: b"Invalid JSON".into(),
                        };
                    };

                    let result = self.config.update(update).await;
                    let result = ConfigUpdateResult::from(result);

                    Response::json(32, result).unwrap()
                }
                _ => Response::MethodNotAllowed((&[Method::Get, Method::Patch]).into()),
            },
            _ => Response::NotFound,
        }
    }
}

#[task]
async fn status(status: &'static StatusSignal) {
    loog::info!("Starting status()");

    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        ticker.next().await;

        status.signal(Status {
            battery_voltage: 0,
            idle_time: 0.0,
            timing_drift: 0.0,
        });
    }
}
