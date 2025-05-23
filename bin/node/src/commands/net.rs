//! Net Subcommand

use crate::flags::{GlobalArgs, MetricsArgs, P2PArgs, RpcArgs};
use clap::Parser;
use kona_p2p::{NetRpcRequest, NetworkBuilder, NetworkRpc};
use kona_rpc::{OpP2PApiServer, RpcConfig};
use tracing::{debug, info, warn};

/// The `net` Subcommand
///
/// The `net` subcommand is used to run the networking stack for the `kona-node`.
///
/// # Usage
///
/// ```sh
/// kona-node net [FLAGS] [OPTIONS]
/// ```
#[derive(Parser, Debug, Clone)]
#[command(about = "Runs the networking stack for the kona-node.")]
pub struct NetCommand {
    /// P2P CLI Flags
    #[command(flatten)]
    pub p2p: P2PArgs,
    /// RPC CLI Flags
    #[command(flatten)]
    pub rpc: RpcArgs,
}

impl NetCommand {
    /// Initializes the telemetry stack and Prometheus metrics recorder.
    pub fn init_telemetry(&self, args: &GlobalArgs, metrics: &MetricsArgs) -> anyhow::Result<()> {
        // Filter out discovery warnings since they're very very noisy.
        let filter = tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("discv5=error".parse()?);

        // Initialize the telemetry stack.
        args.init_tracing(Some(filter))?;
        metrics.init_metrics()
    }

    /// Run the Net subcommand.
    pub async fn run(self, args: &GlobalArgs) -> anyhow::Result<()> {
        let signer = args.genesis_signer()?;
        info!("Genesis block signer: {:?}", signer);

        // Setup the RPC server with the P2P RPC Module
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        let p2p_module = NetworkRpc::new(tx.clone()).into_rpc();
        let rpc_config = RpcConfig::from(&self.rpc);
        let mut launcher = rpc_config.as_launcher().merge(p2p_module)?;
        let handle = launcher.start().await?;
        info!("Started RPC server on {:?}:{}", rpc_config.listen_addr, rpc_config.listen_port);

        // Get the rollup config from the args
        let rollup_config = args
            .rollup_config()
            .ok_or(anyhow::anyhow!("Rollug config not found for chain id: {}", args.l2_chain_id))?;

        // Start the Network Stack
        self.p2p.check_ports()?;
        let p2p_config = self.p2p.config(&rollup_config, args, None).await?;
        let mut network = NetworkBuilder::from(p2p_config)
            .with_chain_id(args.l2_chain_id)
            .with_rpc_receiver(rx)
            .with_rollup_config(rollup_config)
            .build()?;
        let mut recv = network.unsafe_block_recv();
        network.start()?;
        info!("Network started, receiving blocks.");

        // On an interval, use the rpc tx to request stats about the p2p network.
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));

        loop {
            tokio::select! {
                payload = recv.recv() => {
                    match payload {
                        Ok(payload) => info!("Received unsafe payload: {:?}", payload.payload_hash),
                        Err(e) => debug!("Failed to receive unsafe payload: {:?}", e),
                    }
                }
                _ = interval.tick() => {
                    let (otx, mut orx) = tokio::sync::oneshot::channel();
                    if let Err(e) = tx.send(NetRpcRequest::PeerCount(otx)).await {
                        warn!("Failed to send network rpc request: {:?}", e);
                        continue;
                    }
                    tokio::time::timeout(tokio::time::Duration::from_secs(5), async move {
                        loop {
                            match orx.try_recv() {
                                Ok((d, g)) => {
                                    let d = d.unwrap_or_default();
                                    info!("Peer counts: Discovery={} | Swarm={}", d, g);
                                    break;
                                }
                                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                                    /* Keep trying to receive */
                                }
                                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                                    break;
                                }
                            }
                        }
                    }).await.unwrap();
                }
                _ = handle.clone().stopped() => {
                    warn!("RPC server stopped");
                    return Ok(());
                }
            }
        }
    }
}
