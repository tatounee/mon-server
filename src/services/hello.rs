use std::{
    future::{Ready, ready},
    task::Poll,
};

use bytes::{Bytes, BytesMut};
use color_eyre::eyre::Report;
use http::{Method, Request, Response};
use tower::Service;

#[derive(Clone)]
pub struct HelloService;

impl Service<Request<BytesMut>> for HelloService {
    type Response = Response<Bytes>;

    type Error = Report;

    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<BytesMut>) -> Self::Future {
        if req.method() == Method::POST {
            let body = format!(
                "Hello {}",
                str::from_utf8(req.body())
                    .unwrap_or("<invalide string>")
                    .to_owned()
            );
            let body = Bytes::from(body);

            let response = Response::builder()
                .status(200)
                .body(body)
                .map_err(Report::new);
            ready(response)
        } else {
            let response = Response::builder()
                .status(200)
                .body(Bytes::from_static(b"Don't you have any name ?"))
                .map_err(Report::new);
            ready(response)
        }
    }
}
