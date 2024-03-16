pub struct HttpHandler;

impl<State, PathParameters> picoserve::routing::MethodHandler<State, PathParameters>
    for HttpHandler
{
    async fn call_method_handler<
        R: picoserve::io::Read,
        W: picoserve::response::ResponseWriter<Error = R::Error>,
    >(
        &self,
        _state: &State,
        _path_parameters: PathParameters,
        _request: picoserve::request::Request<'_, R>,
        _response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        todo!()
    }
}
