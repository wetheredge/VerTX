pub(crate) mod api;

use core::future::Future;

use embassy_net::driver::Driver;
use embassy_net::tcp::TcpSocket;
use embassy_time::Duration;
use picoserve::response::ResponseWriter;
use picoserve::routing::{PathRouterService, RequestHandlerService};
use picoserve::{self, Config, ResponseSent};

use crate::{Api, Context};

const TCP_BUFFER: usize = 1024;
const HTTP_BUFFER: usize = 2048;

include!(concat!(env!("OUT_DIR"), "/configurator.rs"));

type RouterError<'a> = picoserve::Error<<TcpSocket<'a> as picoserve::io::Socket>::Error>;
pub trait Router<A: Api>: seal::Seal {
    fn serve<'a>(
        &self,
        config: &Config<Duration>,
        buffer: &mut [u8],
        socket: TcpSocket<'a>,
        api: &A,
    ) -> impl Future<Output = Result<(), RouterError<'a>>>;
}

mod seal {
    pub trait Seal {}
}

impl<A: Api, PR: picoserve::routing::PathRouter<A>> seal::Seal for picoserve::Router<PR, A> {}
impl<A: Api, PR: picoserve::routing::PathRouter<A>> Router<A> for picoserve::Router<PR, A> {
    async fn serve<'a>(
        &self,
        config: &Config<Duration>,
        buffer: &mut [u8],
        socket: TcpSocket<'a>,
        api: &A,
    ) -> Result<(), RouterError<'a>> {
        picoserve::serve_with_state(self, config, buffer, socket, api)
            .await
            .map(|_| ())
    }
}

pub struct AssetsRouter;

impl<A: Api> PathRouterService<A> for AssetsRouter {
    async fn call_request_handler_service<
        R: picoserve::io::Read,
        W: ResponseWriter<Error = R::Error>,
    >(
        &self,
        api: &A,
        _current_path_parameters: (),
        path: picoserve::request::Path<'_>,
        request: picoserve::request::Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        let path = path.encoded();
        let file = if let Ok(asset) = ASSETS.binary_search_by_key(&path, |(route, _)| route) {
            &ASSETS[asset].1
        } else {
            &INDEX
        };
        file.call_request_handler_service(api, (), request, response_writer)
            .await
    }
}

static CONFIG: Config<Duration> = Config {
    timeouts: picoserve::Timeouts {
        start_read_request: Some(Duration::from_secs(5)),
        read_request: Some(Duration::from_secs(1)),
        write: Some(Duration::from_secs(2)),
    },
    connection: picoserve::KeepAlive::KeepAlive,
};

pub async fn worker<A: Api, D: Driver>(
    id: usize,
    context: &Context<D>,
    router: &impl Router<A>,
    api: &A,
) -> ! {
    let Context(stack) = context;

    let mut rx_buffer = [0; TCP_BUFFER];
    let mut tx_buffer = [0; TCP_BUFFER];

    loop {
        let mut tcp = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

        if let Err(err) = tcp.accept(80).await {
            log::warn!("server({id}): Accept error: {err:?}");
            continue;
        }

        if let Err(err) = router.serve(&CONFIG, &mut [0; HTTP_BUFFER], tcp, api).await {
            log::error!("server({id}): Error: {err:?}");
        }
    }
}
