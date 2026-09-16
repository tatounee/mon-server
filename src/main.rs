#![feature(impl_trait_in_assoc_type)]
#![feature(trim_prefix_suffix)]
#![feature(normalize_lexically)]
#![warn(clippy::pedantic)]
// #![allow(warnings)]

use color_eyre::{Result, eyre::Context};
use dotenvy::dotenv;
use tokio::{fs::File, net::TcpListener};
use tower::ServiceBuilder;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::{
    body::Body,
    services::{DatabaseLayer, HelloService, Router, StaticFile},
};

mod app;
mod body;
mod config;
mod error;
mod services;
mod typed_map;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv()?;

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    color_eyre::install()?;

    let (host, port) = config::from_env()?;
    info!("Starting application on http://[{host}]:{port}");

    let listener = TcpListener::bind((host, port)).await?;

    let static_dir =
        std::env::var("STATIC_DIR").wrap_err("reading STATIC_DIR environement variable")?;

    let hello = ServiceBuilder::new()
        .layer(DatabaseLayer)
        .service(HelloService);

    let router = Router::new()
        .layer(DatabaseLayer)
        .route("/hello", hello)
        .route("/static", StaticFile::new(static_dir)?);

    app::run(listener, router).await?;

    Ok(())
}
