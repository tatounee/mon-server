use bytes::BytesMut;
use color_eyre::Result;
use httparse::Status;
use tokio::net::TcpListener;
use tracing::{Instrument, Span, debug, error, info, info_span};

pub async fn run(tcp: TcpListener) -> Result<()> {
    use tokio::io::AsyncReadExt;

    loop {
        let (mut client, client_addr) = tcp.accept().await?;

        let span = info_span!("client", addr = %client_addr);
        let _guard = span.enter();

        info!("Open connection");

        tokio::spawn(
            async move {
                let (mut client_read, _client_write) = client.split();

                let mut blank_buf = BytesMut::with_capacity(1024);
                let mut filled_buf = blank_buf.split();

                debug!(
                    blank_buf = blank_buf.capacity(),
                    filled_buf = filled_buf.capacity()
                );

                loop {
                    match client_read.read_buf(&mut blank_buf).await {
                        // socket closed
                        Ok(0) => {
                            info!("Close connection");
                            return Ok::<_, color_eyre::Report>(());
                        }
                        Ok(n) => info!("Read {n} bytes"),
                        Err(e) => {
                            error!("failed to read from socket; err = {:?}", e);
                            return Ok(());
                        }
                    };

                    let new_data = blank_buf.split();
                    filled_buf.unsplit(new_data);

                    let mut headers = [httparse::EMPTY_HEADER; 64];
                    let mut request = httparse::Request::new(&mut headers);
                    let res = request.parse(&filled_buf)?;

                    if matches!(res, Status::Complete(_)) {
                        debug!(?request);
                        break Ok(());
                    }
                }
            }
            .instrument(Span::current()),
        );
    }
}
