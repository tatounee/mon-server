use bytes::Bytes;
use http::{Response, StatusCode};

use crate::body::Body;

pub fn basic_response<T>(code: T) -> Response<Body>
where
    T: TryInto<StatusCode>,
    <T as TryInto<StatusCode>>::Error: Into<http::Error>,
{
    Response::builder()
        .status(code)
        .body(Body::Static(Bytes::new()))
        .unwrap()
}
