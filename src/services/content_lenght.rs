use http::{HeaderValue, Response, header};
use tower::{Layer, Service};

use crate::body::Body;

pub struct ContentLength<S> {
    inner: S,
}

impl<Req, S> Service<Req> for ContentLength<S>
where
    S: Service<Req, Response = Response<Body>>,
{
    type Response = Response<Body>;

    type Error = S::Error;

    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let res = self.inner.call(req);
        async move {
            let mut res = res.await?;

            if let Body::Static(body) = res.body() {
                let lenght = body.len();

                *res.headers_mut()
                    .entry(header::CONTENT_LENGTH)
                    .or_insert(HeaderValue::from_static("")) = HeaderValue::from(lenght);
            }

            Ok(res)
        }
    }
}

pub struct ContentLengthLayer;

impl<S> Layer<S> for ContentLengthLayer {
    type Service = ContentLength<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ContentLength { inner }
    }
}
