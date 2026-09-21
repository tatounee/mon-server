use std::task::Poll;

use bytes::{Bytes, BytesMut};
use color_eyre::eyre::Report;
use http::{Method, Request, Response};
use tower::Service;

use crate::{body::Body, services::DbHandler, typed_map::Value};

#[derive(Clone)]
pub struct HelloService;

impl Service<Request<BytesMut>> for HelloService {
    type Response = Response<Body>;

    type Error = Report;

    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<BytesMut>) -> Self::Future {
        async move {
            if req.method() == Method::POST {
                let name = str::from_utf8(req.body())
                    .unwrap_or("<invalide string>")
                    .to_owned();

                if let Some(db) = req.extensions().get::<DbHandler>() {
                    db.insert::<HelloService>(Value::Empty, Value::String(name.clone()))
                        .await;
                }

                let body = format!("Get {name}");
                Response::builder()
                    .status(200)
                    .body(Body::Static(Bytes::from(body)))
                    .map_err(Report::new)
            } else {
                if let Some(db) = req.extensions().get::<DbHandler>() {
                    let mut name = None;
                    db.get::<HelloService>(Value::Empty, |previous_name| {
                        name = previous_name.cloned();
                    })
                    .await;

                    if let Some(name) = name {
                        let body = format!("Hello {}, how are you ?", name.as_string().unwrap());
                        return Response::builder()
                            .status(200)
                            .body(Body::Static(Bytes::from(body)))
                            .map_err(Report::new);
                    }
                }

                Response::builder()
                    .status(200)
                    .body(Body::Static(Bytes::from_static(
                        b"Don't you have any name ?",
                    )))
                    .map_err(Report::new)
            }
        }
    }
}
