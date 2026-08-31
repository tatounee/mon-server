use bytes::BytesMut;
use color_eyre::Result;
use httparse::Status;
use hyper::client;
use tokio::{io::BufReader, net::TcpListener};
use tracing::{Instrument, Span, debug, error, info, info_span};

pub async fn run(tcp: TcpListener) -> Result<()> {
    use tokio::io::AsyncBufReadExt;

    loop {
        let (mut client, client_addr) = tcp.accept().await?;

        let span = info_span!("client", addr = %client_addr);
        let _guard = span.enter();

        info!("Open connection");

        tokio::spawn(
            async move {
                let (client_read, _client_write) = client.split();
                let mut client_read = BufReader::new(client_read);

                loop {
                    // TODO: Marche pas, parce qu'une fois `fill_buf` appeler, il renvoi toujours la même chose sans
                    //       read à nouveau depuis son reader intern...
                    let buf = match client_read.fill_buf().await {
                        // socket closed
                        Ok(&[]) => {
                            info!("Close connection");
                            return Ok::<_, color_eyre::Report>(());
                        }
                        Ok(buf) => buf,
                        Err(e) => {
                            error!("failed to read from socket; err = {:?}", e);
                            return Ok(());
                        }
                    };

                    {
                        let mut headers = [httparse::EMPTY_HEADER; 64];
                        let mut request = httparse::Request::new(&mut headers);
                        let res = request.parse(buf)?;

                        info!(?res);

                        let amt;
                        match res {
                            Status::Complete(n) => {
                                amt = n;
                                debug!(?request);
                            }
                            Status::Partial => continue,
                        }
                        client_read.consume(amt);
                    }

                    // Write the data back
                    // if let Err(e) = client.write_all(&blank_buf[0..n]).await {
                    //     error!("failed to write to socket; err = {:?}", e);
                    //     return Ok(());
                    // }
                }
            }
            .instrument(Span::current()),
        );
    }
}
