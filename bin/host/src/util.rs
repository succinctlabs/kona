//! Contains utility functions and helpers for the host program.

use alloy_primitives::{hex, Bytes};
use alloy_provider::ReqwestProvider;
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;
use anyhow::{anyhow, Result};
use kona_proof::HintType;
use os_pipe::{PipeReader, PipeWriter};
use reqwest::Client;
use tokio::task::JoinHandle;

/// A bidirectional pipe, with a client and host end.
#[derive(Debug)]
pub struct BidirectionalPipe {
    pub(crate) client: Pipe,
    pub(crate) host: Pipe,
}

/// A single-direction pipe, with a read and write end.
#[derive(Debug)]
pub struct Pipe {
    pub(crate) read: PipeReader,
    pub(crate) write: PipeWriter,
}

/// Creates a [BidirectionalPipe] instance.
pub fn bidirectional_pipe() -> Result<BidirectionalPipe> {
    let (ar, bw) = os_pipe::pipe().map_err(|e| anyhow!("Failed to create pipe: {e}"))?;
    let (br, aw) = os_pipe::pipe().map_err(|e| anyhow!("Failed to create pipe: {e}"))?;

    Ok(BidirectionalPipe {
        client: Pipe { read: ar, write: aw },
        host: Pipe { read: br, write: bw },
    })
}

/// Parses a hint from a string.
///
/// Hints are of the format `<hint_type> <hint_data>`, where `<hint_type>` is a string that
/// represents the type of hint, and `<hint_data>` is the data associated with the hint
/// (bytes encoded as hex UTF-8).
pub(crate) fn parse_hint(s: &str) -> Result<(HintType, Bytes)> {
    let mut parts = s.split(' ').collect::<Vec<_>>();

    if parts.len() != 2 {
        anyhow::bail!("Invalid hint format: {}", s);
    }

    let hint_type = HintType::try_from(parts.remove(0))?;
    let hint_data = hex::decode(parts.remove(0)).map_err(|e| anyhow!(e))?.into();

    Ok((hint_type, hint_data))
}

/// Returns an HTTP provider for the given URL.
pub(crate) fn http_provider(url: &str) -> ReqwestProvider {
    let url = url.parse().unwrap();
    let http = Http::<Client>::new(url);
    ReqwestProvider::new(RpcClient::new(http, true))
}

/// Flattens the result of a [JoinHandle] into a single result.
pub(crate) async fn flatten_join_result<T, E>(
    handle: JoinHandle<Result<T, E>>,
) -> Result<T, anyhow::Error>
where
    E: std::fmt::Display,
{
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(anyhow!("{}", err)),
        Err(err) => anyhow::bail!(err),
    }
}
