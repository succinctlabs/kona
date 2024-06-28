mod cli;
mod fetcher;
mod kv;
mod server;
mod types;
mod util;

use kona_host::{
    start_server, start_server_and_native_client, init_tracing_subscriber, HostCli
};
use tracing::info;
use anyhow::Result;
use clap::Parser;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cfg = HostCli::parse();
    init_tracing_subscriber(cfg.v)?;

    if cfg.server {
        start_server(cfg).await?;
    } else {
        start_server_and_native_client(cfg).await?;
    }

    info!("Exiting host program.");
    Ok(())
}
