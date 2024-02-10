pub struct HttpHandler;

impl<State, PathParameters> picoserve::routing::MethodHandler<State, PathParameters>
    for HttpHandler
{
    async fn call_method_handler<W: picoserve::response::ResponseWriter>(
        &self,
        _state: &State,
        _path_parameters: PathParameters,
        _request: picoserve::request::Request<'_>,
        _response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        todo!()
    }
}
