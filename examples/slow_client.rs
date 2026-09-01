//! Client de test qui envoie une requête HTTP en plusieurs morceaux, avec un
//! délai entre chaque, pour voir comment le serveur réagit quand il ne reçoit
//! pas tout d'un coup.
//!
//! Usage : `cargo run --example slow_client`

use std::net::{Ipv6Addr, SocketAddr};
use std::time::Duration;

use color_eyre::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;

const ADDR: SocketAddr = SocketAddr::new(std::net::IpAddr::V6(Ipv6Addr::LOCALHOST), 8800);

/// Délai artificiel entre deux morceaux de la requête.
const CHUNK_DELAY: Duration = Duration::from_millis(700);

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let chunks: [&str; 6] = [
        "GET / HTTP/1.1\r\n",
        "Host: [::1]:8800\r\n",
        "User-Ag",
        "ent: rust\r\n",
        "\r\nGET / HTTP/1.1\r\n",
        "Host: you.fr\n\r\n\r",
    ];

    println!("Connexion à {ADDR}…");
    let mut stream = TcpStream::connect(ADDR).await?;
    stream.set_nodelay(true)?;
    println!("Connecté.");

    for (i, chunk) in chunks.iter().enumerate() {
        stream.write_all(chunk.as_bytes()).await?;
        stream.flush().await?;
        println!("[{}/{}] envoyé : {chunk:?}", i + 1, chunks.len());

        if i + 1 < chunks.len() {
            sleep(CHUNK_DELAY).await;
        }
    }

    println!("Requête complète envoyée, lecture de la réponse…");

    let mut response = Vec::new();
    stream.read_to_end(&mut response).await?;

    println!("--- réponse ({} octets) ---", response.len());
    println!("{}", String::from_utf8_lossy(&response));

    Ok(())
}
