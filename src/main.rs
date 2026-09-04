#![allow(warnings)]
#![feature(impl_trait_in_assoc_type)]
#![feature(impl_trait_in_fn_trait_return)]
#![feature(impl_trait_in_bindings)]

use color_eyre::Result;
use dotenvy::dotenv;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod app;
mod config;
mod error;
mod services;

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

    // let app = root();

    let listener = TcpListener::bind((host, port)).await.unwrap();
    // axum::serve(listener, app).await.unwrap();

    app::run(listener).await?;

    Ok(())
}
