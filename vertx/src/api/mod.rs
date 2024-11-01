mod protocol;

use embassy_executor::{task, Spawner};
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};
use static_cell::ConstStaticCell;

use self::protocol::{Request, Response};
use crate::build_info;

pub(crate) type BatterySignal = Signal<crate::mutex::SingleCore, u16>;

pub(crate) struct Api {
    reset: &'static crate::reset::Manager,
    config: &'static crate::config::Manager,

    battery: &'static BatterySignal,
}

impl Api {
    pub(crate) fn new(
        spawner: Spawner,
        reset: &'static crate::reset::Manager,
        config: &'static crate::config::Manager,
    ) -> Self {
        static BATTERY: ConstStaticCell<BatterySignal> = ConstStaticCell::new(Signal::new());
        let battery = BATTERY.take();

        spawner.must_spawn(mock_battery(battery));

        Self {
            reset,
            config,

            battery,
        }
    }
}

impl vertx_network::Api for Api {
    type Buffer = [u8; 256];

    fn buffer() -> Self::Buffer {
        [0; 256]
    }

    async fn next_response<'b>(&self, buffer: &'b mut Self::Buffer) -> &'b [u8] {
        encode(Response::Vbat(self.battery.wait().await), buffer)
    }

    async fn handle<'b>(&self, request: &[u8], buffer: &'b mut Self::Buffer) -> Option<&'b [u8]> {
        let request = match postcard::from_bytes(request) {
            Ok(request) => request,
            Err(err) => {
                loog::error!("Failed to parse request: {err:?}");
                return None;
            }
        };

        // FIXME: Request needs defmt::Format impl
        // loog::debug!("Received api request: {request:?}");

        let response = match request {
            Request::BuildInfo => Some(Response::BuildInfo {
                target: build_info::TARGET,
                version: build_info::VERSION,
                debug: build_info::DEBUG,
                git_branch: build_info::GIT_BRANCH,
                git_commit: build_info::GIT_COMMIT,
                git_dirty: build_info::GIT_DIRTY,
            }),
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
            Request::Config => {
                let mut buffer = alloc::vec![0; crate::config::BYTE_LENGTH];
                let len = self.config.serialize(&mut buffer).unwrap();
                buffer.truncate(len);
                Some(Response::Config(buffer.into()))
            }
            Request::UpdateConfig { id, update } => {
                let result = self.config.update(update).await.into();
                Some(Response::ConfigUpdate { id, result })
            }
        };

        response.map(|response| encode(response, buffer))
    }
}

fn encode(response: Response, buffer: &mut [u8]) -> &[u8] {
    match postcard::to_slice(&response, buffer) {
        Ok(data) => data,
        Err(err) => panic!("Failed to encode api response: {err}"),
    }
}

#[task]
async fn mock_battery(battery: &'static BatterySignal) {
    loog::info!("Starting mock_battery()");

    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        ticker.next().await;
        battery.signal(0);
    }
}
