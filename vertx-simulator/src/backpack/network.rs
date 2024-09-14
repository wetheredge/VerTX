use std::pin::Pin;
use std::sync::Arc;
use std::{env, io, task};

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_util::sync::CancellationToken;
use vertx_backpack_ipc::ToMain;

struct Config {
    host: Option<String>,
    port: u16,
    vite_host: Option<String>,
    vite_port: u16,
}

impl Config {
    fn from_env() -> Self {
        let port = |var, def| env::var(var).map(|p| p.parse().unwrap()).unwrap_or(def);

        Self {
            host: env::var("VERTX_HOST").ok(),
            port: port("VERTX_PORT", 8080),
            vite_host: env::var("VITE_HOST").ok(),
            vite_port: port("VERTX_PORT", 5173),
        }
    }

    fn addr(&self) -> (&str, u16) {
        (self.host.as_deref().unwrap_or("localhost"), self.port)
    }

    fn vite_addr(&self) -> (&str, u16) {
        let host = self.vite_host.as_deref().unwrap_or("localhost");
        (host, self.vite_port)
    }
}

pub(super) async fn start(
    cancel: CancellationToken,
    mut tx: super::Tx,
    responses: Arc<Mutex<mpsc::UnboundedReceiver<Vec<u8>>>>,
) {
    let responses = Arc::new(responses);

    let config = Config::from_env();

    let (host, port) = config.addr();
    let listener = TcpListener::bind((host, port)).await.unwrap();
    println!("Listening on http://{host}:{port}");

    let (host, port) = config.vite_addr();
    println!("Proxying http://{host}:{port}");

    super::send(&mut tx, ToMain::NetworkUp);

    loop {
        let (socket, _) = tokio::select! {
            () = cancel.cancelled() => break,
            r = listener.accept() => r.unwrap(),
        };

        let cancel = cancel.clone();
        let mut tx = tx.clone();
        let responses = Arc::clone(&responses);
        let host = host.to_owned();
        tokio::spawn(async move {
            let mut socket = BufReader::new(socket);
            let mut peeked = Vec::new();
            socket.read_until(b'\n', &mut peeked).await.unwrap();
            // This allows /api/* as well, while the embedded version does not
            let is_api = peeked.get(0..8).is_some_and(|x| x == b"GET /api");
            let mut socket = PeekedStream::new(peeked, &mut socket);

            if is_api {
                let mut responses = responses.lock().await;
                let mut api = tokio_tungstenite::accept_async(socket).await.unwrap();

                loop {
                    let response = tokio::select! {
                        () = cancel.cancelled() => break,
                        Some(request) = api.next() => {
                            match request.unwrap() {
                                WsMessage::Binary(request) => {
                                    super::send(&mut tx, ToMain::ApiRequest(request));
                                    continue;
                                }
                                WsMessage::Close(_) => break,
                                WsMessage::Ping(data) => WsMessage::Pong(data),
                                // The configurator shouldn't ever send these
                                WsMessage::Text(_) | WsMessage::Pong(_) => continue,
                                WsMessage::Frame(_) => unreachable!(),
                            }
                        }
                        Some(response) = responses.recv() => WsMessage::Binary(response),
                        else => break,
                    };
                    api.send(response).await.unwrap();
                }
            } else {
                let mut vite = TcpStream::connect((host, port)).await.unwrap();
                tokio::select! {
                    () = cancel.cancelled() => {}
                    Err(err) = tokio::io::copy_bidirectional(&mut socket, &mut vite) => {
                        eprintln!("proxy error: {err:?}");
                    }
                };
            }
        });
    }
}

struct PeekedStream<'a, S> {
    peeked: Vec<u8>,
    stream: Pin<&'a mut S>,
    read_peeked: usize,
}

impl<'a, S: Unpin> PeekedStream<'a, S> {
    fn new(peeked: Vec<u8>, stream: &'a mut S) -> Self {
        Self {
            peeked,
            stream: Pin::new(stream),
            read_peeked: 0,
        }
    }
}

impl<S: AsyncRead> AsyncRead for PeekedStream<'_, S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> task::Poll<io::Result<()>> {
        if self.read_peeked < self.peeked.len() {
            let len = (self.peeked.len() - self.read_peeked).min(buf.remaining());
            let end = self.read_peeked + len;
            buf.put_slice(&self.peeked[self.read_peeked..end]);
            self.read_peeked = end;
            task::Poll::Ready(Ok(()))
        } else {
            self.stream.as_mut().poll_read(cx, buf)
        }
    }
}

impl<S: AsyncWrite> AsyncWrite for PeekedStream<'_, S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &[u8],
    ) -> task::Poll<Result<usize, io::Error>> {
        self.stream.as_mut().poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Result<(), io::Error>> {
        self.stream.as_mut().poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Result<(), io::Error>> {
        self.stream.as_mut().poll_shutdown(cx)
    }
}
