use bytes::{Bytes, BytesMut};
use color_eyre::Result;
use color_eyre::eyre::Report;
use futures::future;
use http::{Request, Response, StatusCode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tower::{Service, ServiceBuilder};
use tracing::{Instrument, Span, error, info, info_span, trace};

use crate::error::ServerError;
use crate::services::ContentLengthLayer;
use crate::services::serde::{HttpSerdeLayer, serialize};
// use crate::services::{ContentLengthLayer, HttpSerdeLayer};
use crate::utils::basic_response;

pub async fn run<S, F>(tcp: TcpListener, service: S) -> Result<()>
where
    S: Service<Request<BytesMut>, Response = Response<Bytes>, Error = Report, Future = F>
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
    S: Service<Request<BytesMut>, Response = Response<Bytes>, Error = Report> + Clone,
    // Res: Into<Bytes>,
    // Err: Debug,
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
            .layer(HttpSerdeLayer)
            .layer(ContentLengthLayer)
            .service(service.clone());

        let ready = future::poll_fn(|cx| service.poll_ready(cx)).await;
        let response = match ready {
            Ok(()) => service.call(&mut filled_buf).await,
            Err(_) => break Ok(()),
        };

        match response {
            Ok(mut response) => {
                client_write.write_all_buf(&mut response).await?;
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
                let mut response = serialize(response).unwrap();
                client_write.write_all_buf(&mut response).await?;
            }
        }
    }
}
