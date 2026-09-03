use std::{
    fmt::{Debug, Display},
    future::{Ready, ready},
    str::FromStr,
    task::Poll,
};

use bytes::BytesMut;
use color_eyre::{Result, eyre::ContextCompat};
use futures::future;
use http::{HeaderName, HeaderValue, Method, Request, Uri, Version};
use httparse::{EMPTY_HEADER, Request as ParsedRequest, Status};
use std::result::Result as StdResult;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tower::Service;
use tracing::{Instrument, Span, debug, error, info, info_span};

pub async fn serve<S, Res, Err>(mut client: TcpStream, mut service: S) -> Result<()>
where
    S: for<'b> Service<Request<&'b [u8]>, Response = Res, Error = Err>,
    Res: Display,
    Err: Debug,
{
    let (mut client_read, _client_write) = client.split();

    let mut blank_buf = BytesMut::with_capacity(1024);
    let mut filled_buf = blank_buf.split();

    loop {
        match client_read.read_buf(&mut blank_buf).await {
            // socket closed
            Ok(0) => {
                info!("Close connection");
                return Ok::<_, color_eyre::Report>(());
            }
            Ok(_) => {}
            Err(e) => {
                error!("failed to read from socket; err = {:?}", e);
                return Ok(());
            }
        };

        let new_data = blank_buf.split();
        filled_buf.unsplit(new_data);

        let mut headers = [EMPTY_HEADER; 64];
        let mut request = ParsedRequest::new(&mut headers);
        let res = request.parse(&filled_buf)?;

        if matches!(res, Status::Complete(_)) {
            let ready = future::poll_fn(|cx| service.poll_ready(cx)).await;
            let request = request2request(request);
            let response = match ready {
                Ok(()) => service.call(request).await,
                Err(_) => break Ok(()),
            };

            match response {
                Ok(response) => debug!("200 {}", response),
                Err(err) => debug!("400 {:?}", err),
            }
            // let request = request2request(request);
            // debug!(response);
            break Ok(());
        }
    }
}

pub async fn run(tcp: TcpListener) -> Result<()> {
    loop {
        let (client, client_addr) = tcp.accept().await?;

        let span = info_span!("client", addr = %client_addr);
        let _guard = span.enter();

        info!("Open connection");

        tokio::spawn(serve(client, HelloService).instrument(Span::current()));
    }
}

fn request2request<'a>(request: ParsedRequest<'a, 'a>) -> Request<&'a [u8]> {
    let method = request
        .method
        .context("parsing method in request2request")
        .map(|method| Method::from_str(method).unwrap())
        .unwrap();

    let uri = request
        .path
        .context("parsing uri in request2request")
        .map(|path| Uri::from_str(path).unwrap())
        .unwrap();

    let version = request
        .version
        .context("parsing version in request2request")
        .map(|minor_version| {
            if minor_version == 0 {
                Version::HTTP_10
            } else {
                Version::HTTP_11
            }
        })
        .unwrap();

    let headers = request.headers.iter().map(|header| {
        (
            HeaderName::from_str(header.name).unwrap(),
            HeaderValue::from_bytes(header.value).unwrap(),
        )
    });

    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .version(version)
        .body(b"body".as_slice())
        .unwrap();

    request.headers_mut().extend(headers);

    request
}

struct HelloService;

impl<'b> Service<Request<&'b [u8]>> for HelloService {
    type Response = String;

    type Error = ();

    type Future = Ready<Result<String, ()>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<StdResult<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<&'b [u8]>) -> Self::Future {
        ready(Ok(format!(
            "Hello {}",
            str::from_utf8(req.body()).unwrap_or("!invalide str!")
        )))
    }
}
