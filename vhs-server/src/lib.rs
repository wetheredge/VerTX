#![no_std]
#![cfg_attr(not(any(feature = "embassy", feature = "tokio")), allow(unused))]

mod assets {
    include!(concat!(env!("OUT_DIR"), "/web_assets.rs"));
}

pub mod api;
mod web_sockets;

use core::future::Future;

pub use picoserve;
use picoserve::routing::{get, PathRouter};
use picoserve::{time, KeepAlive, Router, Timeouts};

type GetDuration<T> = <T as picoserve::Timer>::Duration;
type Result<S> = core::result::Result<u64, picoserve::Error<<S as picoserve::io::Socket>::Error>>;

#[cfg(feature = "embassy")]
type Duration = GetDuration<time::EmbassyTimer>;
#[cfg(feature = "tokio")]
type Duration = GetDuration<time::TokioTimer>;

#[cfg(any(feature = "embassy", feature = "tokio"))]
pub type Config = picoserve::Config<Duration>;

#[cfg(any(feature = "embassy", feature = "tokio"))]
pub const CONFIG: Config = Config {
    timeouts: Timeouts {
        start_read_request: Some(Duration::from_secs(5)),
        read_request: Some(Duration::from_secs(1)),
        write: Some(Duration::from_secs(1)),
    },
    connection: KeepAlive::KeepAlive,
};

pub fn router<S: State>() -> Router<impl PathRouter<S>, S> {
    Router::new()
        .route("/", get(|| assets::INDEX))
        // .route("/favicon.svg", get(|| assets::FAVICON))
        .route("/ws", web_sockets::UpgradeHandler)
}

#[cfg(feature = "embassy")]
type Socket<'a> = embassy_net::tcp::TcpSocket<'a>;

#[cfg(feature = "embassy")]
pub async fn serve<'a, const BUFFER_SIZE: usize, S: State>(
    router: &Router<impl PathRouter<S>, S>,
    config: &Config,
    socket: Socket<'a>,
    state: &S,
) -> Result<Socket<'a>> {
    picoserve::serve_with_state(router, config, &mut [0; BUFFER_SIZE], socket, state).await
}

#[cfg(feature = "tokio")]
type Socket = tokio::net::TcpStream;

#[cfg(feature = "tokio")]
pub async fn serve<const BUFFER_SIZE: usize, S: State>(
    router: &Router<impl PathRouter<S>, S>,
    config: &Config,
    socket: Socket,
    state: &S,
) -> Result<Socket> {
    picoserve::serve_with_state(router, config, &mut [0; BUFFER_SIZE], socket, state).await
}

pub trait State {
    fn handle_request(&self, request: api::Request) -> Option<api::Response>;
    fn next_response(&self) -> impl Future<Output = api::Response>;
}
