use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as Base64;
use edge_ws::FrameType;
use embedded_io_async::{Read, Write};
use sha1::{Digest as _, Sha1};

#[expect(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
enum Status {
    Normal = 1000,
    GoingAway = 1001,
    ProtocolError = 1002,
    UnacceptableType = 1003,
    InvalidData = 1007,
    PolicyViolation = 1008,
    TooBig = 1009,
    RequiredExtension = 1010,
    InternalError = 1011,
}

const MAGIC_KEY: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub(super) async fn run<R, W>(
    mut rx: R,
    mut tx: W,
    api: &crate::configurator::Api,
    headers: &[httparse::Header<'_>],
    connection: Option<&[u8]>,
) -> Result<(), R::Error>
where
    R: Read,
    W: Write<Error = R::Error>,
{
    let connection = connection.is_some_and(|c| {
        c.split(|b| *b == b',')
            .any(|c| c.trim_ascii().eq_ignore_ascii_case(b"upgrade"))
    });
    let upgrade = headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("upgrade"))
        .is_some_and(|h| h.value.eq_ignore_ascii_case(b"websocket"));
    let key = headers.iter().find_map(|h| {
        h.name
            .eq_ignore_ascii_case("sec-websocket-key")
            .then_some(h.value)
    });
    let version = headers.iter().find_map(|h| {
        h.name
            .eq_ignore_ascii_case("sec-websocket-version")
            .then_some(h.value)
    });

    let (true, true, Some(key), Some(version)) = (connection, upgrade, key, version) else {
        super::respond::bad_request(&mut tx, b"Invalid WebSocket upgrade").await?;
        return Ok(());
    };

    if version != b"13" {
        tx.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Type:text/plain\r\nContent-Length:0\r\nSec-WebSocket-Version:25\r\n\r\nInvalid WebSocket version").await?;
    }

    respond_handshake(key, &mut tx).await?;

    let tx = &mut tx;
    let mut api_buffer = crate::configurator::ApiBuffer::new();
    loop {
        const RX_LEN: usize = 1 + 1 + 4 + 125; // header + short len + mask + payload
        const TX_LEN: usize = 0;
        const LEN: usize = if RX_LEN > TX_LEN { RX_LEN } else { TX_LEN };

        let mut buffer = [0; LEN];

        let (frame_type, payload) = match edge_ws::io::recv(&mut rx, &mut buffer).await {
            Ok((frame_type, len)) => (frame_type, &buffer[0..len]),
            Err(edge_ws::Error::Incomplete(_)) => {
                // Only returned if the header cannot fit in the buffer, which cannot happen
                // since, even at max payload length, the header is at most 14 bytes long
                unreachable!()
            }
            Err(edge_ws::Error::Invalid | edge_ws::Error::InvalidLen) => {
                return close(tx, Status::ProtocolError).await;
            }
            Err(edge_ws::Error::BufferOverflow) => return close(tx, Status::TooBig).await,
            Err(edge_ws::Error::Io(err)) => return Err(err),
        };

        match frame_type {
            FrameType::Text(_) => return close(tx, Status::UnacceptableType).await,
            FrameType::Binary(false) => {
                if let Some(response) = api.handle(payload, &mut api_buffer).await {
                    send(tx, FrameType::Binary(false), response).await?;
                }
            }
            FrameType::Ping => send(tx, FrameType::Pong, payload).await?,
            FrameType::Pong => {}
            FrameType::Close => return send(tx, FrameType::Close, payload).await,

            FrameType::Binary(true) | FrameType::Continue(_) => {
                return close(tx, Status::PolicyViolation).await;
            }
        }
    }
}

async fn respond_handshake<W>(key: &[u8], tx: &mut W) -> Result<(), W::Error>
where
    W: Write,
{
    const HASH_LEN: usize = 20;
    const ENCODED_LEN: usize = if let Some(len) = base64::encoded_len(HASH_LEN, true) {
        len
    } else {
        unreachable!()
    };

    tx.write_all(b"HTTP/1.1 101 Switching Protocols\r\nConnection:Upgrade\r\nUpgrade:websocket\r\nSec-WebSocket-Accept:").await?;

    let digest = Sha1::new()
        .chain_update(key.trim_ascii())
        .chain_update(MAGIC_KEY)
        .finalize();
    let mut buffer = [0; ENCODED_LEN];
    let len = Base64.encode_slice(digest, &mut buffer).unwrap();
    let digest = &buffer[0..len];

    tx.write_all(digest).await?;
    tx.write_all(b"\r\n\r\n").await?;
    Ok(())
}

async fn close<W: Write>(tx: &mut W, status: Status) -> Result<(), W::Error> {
    // FIXME: wait for response?

    let payload = (status as u16).to_be_bytes();
    send(tx, FrameType::Close, &payload).await
}

async fn send<W: Write>(tx: &mut W, frame_type: FrameType, payload: &[u8]) -> Result<(), W::Error> {
    edge_ws::io::send(tx, frame_type, None, payload)
        .await
        .map_err(|err| match err {
            edge_ws::Error::Io(err) => err,
            _ => unreachable!(),
        })
}
