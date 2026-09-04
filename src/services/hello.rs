use std::{
    future::{Ready, ready},
    task::Poll,
};

use bytes::BytesMut;
use color_eyre::eyre::Report;
use http::{Method, Request, Response};
use tower::Service;
use tracing::debug;

pub struct HelloService;

impl Service<Request<BytesMut>> for HelloService {
    type Response = Response<String>;

    type Error = Report;

    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<BytesMut>) -> Self::Future {
        match req.method() {
            &Method::POST => {
                let body = format!(
                    "Hello {}",
                    str::from_utf8(req.body())
                        .unwrap_or("!invalide str!")
                        .to_owned()
                );

                let response = Response::builder()
                    .status(200)
                    .body(body)
                    .map_err(Report::new);
                ready(response)
            }
            _ => {
                let response = Response::builder()
                    .status(200)
                    .body("Don't you have any name ?".to_owned())
                    .map_err(Report::new);
                ready(response)
            }
        }
    }
}
