use std::str::FromStr;
use std::task::Poll;

use bytes::{Bytes, BytesMut};
use color_eyre::eyre::{ContextCompat, Report, WrapErr};
use http::{HeaderName, HeaderValue, Method, Request, Response, Uri, Version};
use httparse::{EMPTY_HEADER, Request as ParsedRequest, Status};
use tower::{Layer, Service};

use crate::error::ServerError;

pub struct HttpDeserialize<S> {
    inner: S,
}

impl<'a, S, B> Service<&'a mut BytesMut> for HttpDeserialize<S>
where
    S: Service<Request<Bytes>, Error = Report, Response = Response<B>>,
{
    type Response = Response<B>;

    type Error = Report;

    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req_buf: &'a mut BytesMut) -> Self::Future {
        let inner = parse(req_buf).map(|request| self.inner.call(request));

        async move { inner?.await }
    }
}

pub struct HttpDeserializeLayer;

impl<S> Layer<S> for HttpDeserializeLayer {
    type Service = HttpDeserialize<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpDeserialize { inner }
    }
}

fn parse(buf: &mut BytesMut) -> Result<Request<Bytes>, Report> {
    let mut headers = [EMPTY_HEADER; 64];
    let mut parsed = ParsedRequest::new(&mut headers);
    let status = parsed.parse(buf)?;

    let Status::Complete(cnt) = status else {
        return Err(Report::new(ServerError::PartialRequest));
    };

    let mut request = request2request(&parsed)?;
    let body = buf.split_off(cnt).freeze();
    *request.body_mut() = body;

    Ok(request)
}

fn request2request(request: &ParsedRequest<'_, '_>) -> Result<Request<Bytes>, Report> {
    let method = request
        .method
        .context("missing method in request2request")
        .and_then(|method| {
            Method::from_str(method).wrap_err(format!("parsing method {method:?}"))
        })?;

    let uri = request
        .path
        .context("missing uri in request2request")
        .and_then(|path| Uri::from_str(path).wrap_err(format!("parsing uri {path:?}")))?;

    let version = request
        .version
        .context("missing version in request2request")
        .map(|minor_version| {
            if minor_version == 0 {
                Version::HTTP_10
            } else {
                Version::HTTP_11
            }
        })?;

    let headers = request
        .headers
        .iter()
        .map(|header| {
            let name = HeaderName::from_str(header.name)
                .wrap_err(format!("parsing header name {:?}", header.name))?;
            let value = HeaderValue::from_bytes(header.value)
                .wrap_err(format!("parsing value of header {:?}", header.name))?;

            Ok::<_, Report>((name, value))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .version(version)
        .body(Bytes::new())
        .wrap_err("building request in request2request")?;

    request.headers_mut().extend(headers);

    Ok(request)
}
