use std::fmt::Write;

use bytes::BytesMut;
use color_eyre::Result;
use color_eyre::eyre::Report;
use futures::{future, stream::StreamExt};
use http::{Request, Response, StatusCode, Version};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tower::{Service, ServiceBuilder};
use tracing::{Instrument, Span, error, info_span, trace};

use crate::body::Body;
use crate::error::ServerError;
use crate::services::ContentLengthLayer;
use crate::services::deserialize::HttpDeserializeLayer;
use crate::utils::basic_response;

pub async fn run<S, F>(tcp: TcpListener, service: S) -> Result<()>
where
    S: Service<Request<BytesMut>, Response = Response<Body>, Error = Report, Future = F>
        + Clone
        + Send
        + 'static,
    F: Send + 'static,
{
    loop {
        let (client, client_addr) = tcp.accept().await?;

        let span = info_span!("client", addr = %client_addr);
        let _guard = span.enter();

        trace!("Open connection");

        tokio::spawn(serve(client, service.clone()).instrument(Span::current()));
    }
}

async fn serve<S>(mut client: TcpStream, service: S) -> Result<()>
where
    S: Service<Request<BytesMut>, Response = Response<Body>, Error = Report> + Clone,
{
    let (mut client_read, mut client_write) = client.split();

    let mut blank_buf = BytesMut::with_capacity(1024);
    let mut filled_buf = blank_buf.split();

    loop {
        match client_read.read_buf(&mut blank_buf).await {
            // socket closed
            Ok(0) => {
                trace!("Close connection");
                return Ok::<_, color_eyre::Report>(());
            }
            Ok(_) => {}
            Err(e) => {
                error!("failed to read from socket; err = {:?}", e);
                return Ok(());
            }
        }

        let new_data = blank_buf.split();
        filled_buf.unsplit(new_data);

        let mut service = ServiceBuilder::new()
            .layer(HttpDeserializeLayer)
            .layer(ContentLengthLayer)
            .service(service.clone());

        let ready = future::poll_fn(|cx| service.poll_ready(cx)).await;
        let response = match ready {
            Ok(()) => service.call(&mut filled_buf).await,
            Err(_) => break Ok(()),
        };

        match response {
            Ok(response) => {
                write_response(&mut client_write, response).await?;
                break Ok(());
            }
            Err(err)
                if matches!(
                    err.downcast_ref::<ServerError>(),
                    Some(&ServerError::PartialRequest)
                ) =>
            {
                #[allow(clippy::needless_continue)]
                continue;
            }
            Err(err) => {
                error!(?err);
                let response = basic_response(StatusCode::INTERNAL_SERVER_ERROR);
                let mut response = serialize_headers(&response).unwrap();
                client_write.write_all_buf(&mut response).await?;
            }
        }
    }
}

async fn write_response<T: AsyncWriteExt + Unpin>(
    client: &mut T,
    response: Response<Body>,
) -> Result<(), Report> {
    let mut buf = serialize_headers(&response)?;
    // let mut headers = serialize_headers(&response)?;

    // res_buf.extend_from_slice(response.into_body().into().as_ref());

    match response.into_body() {
        Body::Static(bytes) => {
            buf.extend_from_slice(&bytes);

            client.write_all_buf(&mut buf).await?;

            Ok(())
        }
        Body::Stream(mut stream) => {
            client.write_all_buf(&mut buf).await?;
            while let Some(mut frame) = stream.next().await {
                client.write_all_buf(&mut frame).await?;
            }
            Ok(())
        }
    }
}

pub fn serialize_headers<B>(response: &Response<B>) -> Result<BytesMut, Report> {
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

    Ok(res_buf)
}
