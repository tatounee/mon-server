use color_eyre::{Result, eyre::Context};
use std::net::Ipv6Addr;

pub fn from_env() -> Result<(Ipv6Addr, u16)> {
    let port = std::env::var("PORT").unwrap_or("3000".to_string());
    let port: u16 = port
        .parse()
        .with_context(|| format!("reading variable PORT \"{port}\""))?;

    let host = std::env::var("HOST").unwrap_or("::1".to_string());
    let host: Ipv6Addr = host
        .parse()
        .with_context(|| format!("reading variable HOST \"{host}\""))?;

    Ok((host, port))
}
