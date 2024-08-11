mod protocol;

use embassy_sync::signal::Signal;
pub(crate) use protocol::{response, Request, Response};

use crate::reset;

pub(crate) type StatusSignal = Signal<crate::mutex::SingleCore, response::Status>;

pub(crate) struct Api {
    config: &'static crate::config::Manager,
    status: &'static StatusSignal,
}

impl Api {
    pub(crate) fn new(
        config: &'static crate::config::Manager,
        status: &'static StatusSignal,
    ) -> Self {
        Self { config, status }
    }
}

impl vertx_server::Api for Api {
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
                log::error!("Failed to parse request: {err:?}");
                return None;
            }
        };

        log::debug!("Received api request: {request:?}");

        let response = match request {
            Request::ProtocolVersion => Some(Response::PROTOCOL_VERSION),
            Request::BuildInfo => Some(include!(concat!(env!("OUT_DIR"), "/build_info.rs")).into()),
            Request::PowerOff => {
                reset::shut_down();
                None
            }
            Request::Reboot => {
                reset::reboot();
                None
            }
            Request::ExitConfigurator => {
                reset::reboot_into(crate::BootMode::Standard);
                None
            }
            Request::CheckForUpdate => todo!(),
            Request::GetConfig => {
                let config = self.config.config();
                let config = vertx_config::storage::postcard::to_vec(config).await;
                Some(Response::Config { config })
            }
            Request::ConfigUpdate { id, key, value } => {
                use vertx_config::update::UpdateRef;
                let result = self.config.update_ref(key, value).await.into();
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
