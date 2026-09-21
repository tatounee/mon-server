use std::{
    pin::Pin,
    task::{Poll, ready},
    time::Duration,
};

use bytes::Bytes;
use color_eyre::Report;

use futures::{Stream, StreamExt};
use http::{Response, StatusCode};
use pin_project_lite::pin_project;
use tokio::time::{Instant, Sleep};
use tower::Service;
use tracing::debug;

use crate::body::Body;

#[derive(Clone)]
pub struct CounterService;

impl<R> Service<R> for CounterService {
    type Response = Response<Body>;

    type Error = Report;

    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: R) -> Self::Future {
        async move {
            let response = Response::builder()
                .status(StatusCode::OK)
                .body(Body::Stream(Counter::new().boxed()))
                .unwrap();

            Ok(response)
        }
    }
}

pin_project! {
    struct Counter {
        #[pin]
        sleep: Sleep,
        value: usize,
    }
}

impl Counter {
    fn new() -> Self {
        Self {
            sleep: tokio::time::sleep(Duration::from_secs(1)),
            value: 0,
        }
    }
}

impl Stream for Counter {
    type Item = Bytes;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut().project();

        ready!(this.sleep.as_mut().poll(cx));

        debug!(value = *this.value);

        this.sleep
            .as_mut()
            .reset(Instant::now() + Duration::from_secs(1));

        let old_value = Bytes::copy_from_slice(format!("{}", *this.value).as_bytes());
        *this.value += 1;
        Poll::Ready(Some(old_value))
    }
}
