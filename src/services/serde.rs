use std::fmt::Write;
use std::str::FromStr;
use std::task::Poll;

use bytes::{Bytes, BytesMut};
use color_eyre::eyre::{ContextCompat, Report, WrapErr};
use http::{HeaderName, HeaderValue, Method, Request, Response, Uri, Version};
use httparse::{EMPTY_HEADER, Request as ParsedRequest, Status};
use tower::{Layer, Service};

use crate::error::ServerError;

pub struct HttpSerde<S> {
    inner: S,
}

impl<'a, S, B> Service<&'a mut BytesMut> for HttpSerde<S>
where
    S: Service<Request<BytesMut>, Error = Report, Response = Response<B>>,
    B: Into<Bytes>,
{
    type Response = Bytes;

    type Error = Report;

    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req_buf: &'a mut BytesMut) -> Self::Future {
        // Parsing is synchronous, so it happens here and only the inner future is
        // kept alive: `Self::Future` borrows neither `self` nor `buf`.
        let inner = parse(req_buf).map(|request| self.inner.call(request));

        async move {
            let response = inner?.await?;
            serialize(response)
        }
    }
}

pub struct HttpSerdeLayer;

impl<S> Layer<S> for HttpSerdeLayer {
    type Service = HttpSerde<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpSerde { inner }
    }
}

fn parse(buf: &mut BytesMut) -> Result<Request<BytesMut>, Report> {
    let mut headers = [EMPTY_HEADER; 64];
    let mut parsed = ParsedRequest::new(&mut headers);
    let status = parsed.parse(buf)?;

    let Status::Complete(cnt) = status else {
        return Err(Report::new(ServerError::PartialRequest));
    };

    let mut request = request2request(&parsed)?;
    let body = buf.split_off(cnt);
    *request.body_mut() = body;

    Ok(request)
}

fn request2request(request: &ParsedRequest<'_, '_>) -> Result<Request<BytesMut>, Report> {
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
        .body(BytesMut::new())
        .wrap_err("building request in request2request")?;

    request.headers_mut().extend(headers);

    Ok(request)
}

pub fn serialize<B: Into<Bytes>>(response: Response<B>) -> Result<Bytes, Report> {
    let mut res_buf = BytesMut::new();

    let version = match response.version() {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2.0",
        Version::HTTP_3 => "HTTP/3.0",
        v => return Err(Report::msg(format!("{v:?} HTTP version does not exist"))),
    };

    res_buf.write_fmt(format_args!(
        "{} {} {}\r\n",
        version,
        response.status().as_u16(),
        response.status().canonical_reason().unwrap_or_default(),
    ))?;

    for (header_name, header_value) in response.headers() {
        res_buf.write_fmt(format_args!(
            "{}: {}\r\n",
            header_name,
            header_value.to_str().map_err(Report::new)?
        ))?;
    }

    res_buf.write_str("\r\n")?;
    res_buf.extend_from_slice(response.into_body().into().as_ref());

    Ok(res_buf.freeze())
}
