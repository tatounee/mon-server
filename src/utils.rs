use bytes::Bytes;
use http::{Response, StatusCode};

pub fn basic_response<T>(code: T) -> Response<Bytes>
where
    T: TryInto<StatusCode>,
    <T as TryInto<StatusCode>>::Error: Into<http::Error>,
{
    Response::builder().status(code).body(Bytes::new()).unwrap()
}
