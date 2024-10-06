mod protocol;

use embassy_executor::{task, Spawner};
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};
pub(crate) use protocol::{response, Request, Response};
use static_cell::make_static;

pub(crate) type StatusSignal = Signal<crate::mutex::SingleCore, response::Status>;

pub(crate) struct Api {
    reset: &'static crate::reset::Manager,
    config: &'static crate::config::Manager,
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
    type Buffer = [u8; 256];

    const NAME: &'static str = protocol::NAME;

    fn buffer() -> Self::Buffer {
        [0; 256]
    }

    async fn next_response<'b>(&self, buffer: &'b mut Self::Buffer) -> &'b [u8] {
        encode(self.status.wait().await.into(), buffer)
    }

    async fn handle<'b>(&self, request: &[u8], buffer: &'b mut Self::Buffer) -> Option<&'b [u8]> {
        let request = match postcard::from_bytes(request) {
            Ok(request) => request,
            Err(err) => {
                loog::error!("Failed to parse request: {err:?}");
                return None;
            }
        };

        // FIXME:
        // loog::debug!("Received api request: {request:?}");

        let response = match request {
            Request::ProtocolVersion => Some(Response::PROTOCOL_VERSION),
            Request::BuildInfo => Some(include!(concat!(env!("OUT_DIR"), "/build_info.rs")).into()),
            Request::PowerOff => {
                self.reset.shut_down();
                None
            }
            Request::Reboot => {
                self.reset.reboot();
                None
            }
            Request::ExitConfigurator => {
                self.reset.reboot_into(crate::BootMode::Standard).await;
                None
            }
            Request::CheckForUpdate => todo!(),
            Request::GetConfig => {
                let mut config = bytemuck::allocation::zeroed_slice_box(crate::config::BYTE_LENGTH);
                self.config.serialize(&mut config).unwrap();
                Some(Response::Config { config })
            }
            Request::ConfigUpdate { id, update } => {
                let result = self.config.update(update).await.into();
                Some(Response::ConfigUpdate { id, result })
            }
        };

        response.map(|r| encode(r, buffer))
    }
}

fn encode(response: Response, buffer: &mut [u8]) -> &[u8] {
    match postcard::to_slice(&response, buffer) {
        Ok(data) => data,
        Err(err) => {
            panic!("Failed to encode api response: {err}");
        }
    }
}

#[task]
async fn status(status: &'static StatusSignal) {
    loog::info!("Starting status()");

    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        ticker.next().await;

        status.signal(response::Status {
            battery_voltage: 0,
            idle_time: 0.0,
            timing_drift: 0.0,
        });
    }
}
