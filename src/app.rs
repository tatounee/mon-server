use std::fmt::{Debug, Display};

use bytes::{Bytes, BytesMut};
use color_eyre::Result;
use futures::future;
use http::{Request, Response};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tower::{Layer, Service, ServiceBuilder};
use tracing::{Instrument, Span, debug, error, info, info_span};

use crate::error::ServerError;
use crate::services::{ContentLengthLayer, HelloService, HttpSerde, HttpSerdeLayer};

pub async fn serve<S, Res, Err>(mut client: TcpStream, _service: S) -> Result<()>
where
    S: Service<Request<BytesMut>, Response = Response<Res>, Error = Err>,
    Res: Into<Bytes>,
    Err: Debug,
{
    let (mut client_read, mut client_write) = client.split();

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

        let mut service = ServiceBuilder::new()
            .layer(HttpSerdeLayer)
            .layer(ContentLengthLayer)
            .service(HelloService);

        let ready = future::poll_fn(|cx| service.poll_ready(cx)).await;
        let mut response = match ready {
            Ok(()) => service.call(&mut filled_buf).await,
            Err(_) => break Ok(()),
        };

        match response {
            Ok(mut response) => {
                client_write.write_all_buf(&mut response).await;
                break Ok(());
            }
            Err(err)
                if matches!(
                    err.downcast_ref::<ServerError>(),
                    Some(&ServerError::PartialRequest)
                ) =>
            {
                continue;
            }
            Err(err) => {
                error!(?err);
                // client_write.write_all_buf(&mut Res).await;
            }
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
