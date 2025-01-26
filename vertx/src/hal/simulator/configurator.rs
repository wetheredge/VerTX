use std::boxed::Box;
use std::convert::Infallible;
use std::string::String;
use std::vec::Vec;

use embassy_sync::channel::Channel;
use wasm_bindgen::prelude::*;

use crate::configurator::api::{self, ContentType, Method};

static REQUESTS: Channel<crate::mutex::MultiCore, Request, 10> = Channel::new();

#[wasm_bindgen(js_name = "Method")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmMethod {
    Get = "GET",
    Post = "POST",
    Delete = "DELETE",
}

impl From<WasmMethod> for Method {
    fn from(method: WasmMethod) -> Self {
        match method {
            WasmMethod::Get => Self::Get,
            WasmMethod::Post => Self::Post,
            WasmMethod::Delete => Self::Delete,
            WasmMethod::__Invalid => loog::unreachable!(),
        }
    }
}

#[derive(Debug)]
pub(super) struct Request {
    pub(super) id: u32,
    pub(super) route: String,
    pub(super) method: WasmMethod,
    pub(super) body: Box<[u8]>,
}

impl Request {
    pub(super) fn push(self) {
        REQUESTS.try_send(self).unwrap();
    }
}

impl api::Request for Request {
    fn method(&self) -> Method {
        self.method.into()
    }

    fn route(&self) -> &str {
        &self.route
    }

    fn body(&self) -> &[u8] {
        &self.body
    }
}

pub(super) struct Configurator;

impl crate::hal::traits::Configurator for Configurator {
    type Request = Request;
    type Writer = Response;

    async fn start(&mut self) {
        super::ipc::open_configurator();
    }

    async fn receive(&mut self) -> (Self::Request, Self::Writer) {
        let request = REQUESTS.receive().await;
        let id = request.id;
        (request, Response::new(id))
    }
}

#[derive(Debug)]
pub(super) struct Response {
    id: u32,
    status: u16,
    json: bool,
    body: Vec<u8>,
    sent: bool,
}

impl Response {
    fn new(id: u32) -> Self {
        Self {
            id,
            status: 0,
            json: false,
            body: Vec::new(),
            sent: false,
        }
    }

    fn set_content_type(&mut self, typ: ContentType) {
        match typ {
            ContentType::Json => self.json = true,
            ContentType::OctetStream => self.json = false,
        }
    }

    fn send(mut self) {
        super::ipc::api_tx(self.id, self.status, self.json, &self.body);
        self.sent = true;
    }
}

#[cfg(debug_assertions)]
impl Drop for Response {
    fn drop(&mut self) {
        if !self.sent {
            loog::panic!("Response was dropped without being sent");
        }
    }
}

impl api::WriteResponse for Response {
    type BodyWriter = Self;
    type ChunkedBodyWriter = Self;
    type Error = Infallible;

    async fn method_not_allowed(self, allow: &str) -> Result<(), Self::Error> {
        todo!("allow: {allow}")
    }

    async fn not_found(mut self) -> Result<(), Self::Error> {
        self.status = 404;
        self.send();
        Ok(())
    }

    async fn service_unavailable(mut self) -> Result<(), Self::Error> {
        self.status = 503;
        self.send();
        Ok(())
    }

    async fn ok_empty(mut self) -> Result<(), Self::Error> {
        self.status = 200;
        self.send();
        Ok(())
    }

    async fn ok_with_len(
        mut self,
        typ: ContentType,
        _len: usize,
    ) -> Result<Self::BodyWriter, Self::Error> {
        self.status = 200;
        self.set_content_type(typ);
        Ok(self)
    }

    async fn ok_chunked(
        mut self,
        typ: ContentType,
    ) -> Result<Self::ChunkedBodyWriter, Self::Error> {
        self.status = 200;
        self.set_content_type(typ);
        Ok(self)
    }
}

impl embedded_io_async::ErrorType for Response {
    type Error = Infallible;
}

impl embedded_io_async::Write for Response {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.body.extend_from_slice(buf);
        Ok(buf.len())
    }
}

impl api::WriteBody for Response {
    async fn finish(self) -> Result<(), Self::Error> {
        self.send();
        Ok(())
    }
}

impl api::WriteChunkedBody for Response {
    type Error = Infallible;

    async fn write(&mut self, chunk: &[&[u8]]) -> Result<(), Self::Error> {
        for chunk in chunk {
            self.body.extend_from_slice(chunk);
        }
        Ok(())
    }

    async fn finish(self) -> Result<(), Self::Error> {
        self.send();
        Ok(())
    }
}
