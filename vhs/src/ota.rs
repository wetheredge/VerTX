use picoserve::routing::MethodHandler;
use vhs_server::picoserve;

pub struct HttpHandler;

impl<State, PathParameters> MethodHandler<State, PathParameters> for HttpHandler {
    async fn call_method_handler<W: picoserve::response::ResponseWriter>(
        &self,
        _state: &State,
        _path_parameters: PathParameters,
        request: picoserve::request::Request<'_>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        todo!()
    }
}
