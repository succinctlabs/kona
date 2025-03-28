//! Standard implementation of the [RollupNode] service, using the governance approved
//! OP Stack configuration of components.
//!
//! See: <https://specs.optimism.io/protocol/rollup-node.html>

use super::{NodeMode, RollupNodeService, SequencerNodeService, ValidatorNodeService};
use crate::{L1WatcherRpc, L2ForkchoiceState, SyncStartError, find_starting_forkchoice};
use alloy_provider::RootProvider;
use async_trait::async_trait;
use kona_derive::{errors::PipelineErrorKind, traits::ChainProvider};
use kona_genesis::RollupConfig;
use kona_protocol::BlockInfo;
use kona_providers_alloy::{
    AlloyChainProvider, AlloyChainProviderError, AlloyL2ChainProvider, OnlineBeaconClient,
    OnlineBlobProvider, OnlinePipeline,
};
use op_alloy_network::Optimism;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use tracing::info;

mod builder;
pub use builder::RollupNodeBuilder;

/// The size of the cache used in the derivation pipeline's providers.
const DERIVATION_PROVIDER_CACHE_SIZE: usize = 1024;

/// The standard implementation of the [RollupNode] service, using the governance approved OP Stack
/// configuration of components.
#[derive(Debug)]
pub struct RollupNode {
    /// The rollup configuration.
    pub(crate) config: Arc<RollupConfig>,
    /// The mode of operation for the node.
    pub(crate) mode: NodeMode,
    /// The L1 EL provider.
    pub(crate) l1_provider: RootProvider,
    /// The L1 beacon API.
    pub(crate) l1_beacon: OnlineBeaconClient,
    /// The L2 EL provider.
    pub(crate) l2_provider: RootProvider<Optimism>,
    /// The L2 engine.
    ///
    /// TODO: Place L2 Engine API client here once it's ready.
    pub(crate) _l2_engine: (),
}

impl RollupNode {
    /// Creates a new [RollupNodeBuilder], instantiated with the given [RollupConfig].
    pub fn builder(config: RollupConfig) -> RollupNodeBuilder {
        RollupNodeBuilder::new(config)
    }
}

#[async_trait]
impl RollupNodeService for RollupNode {
    fn mode(&self) -> NodeMode {
        self.mode
    }
}

#[async_trait]
impl ValidatorNodeService for RollupNode {
    type DataAvailabilityWatcher = L1WatcherRpc;
    type DerivationPipeline = OnlinePipeline;
    type Error = RollupNodeError;

    fn config(&self) -> &RollupConfig {
        &self.config
    }

    fn new_da_watcher(
        &self,
        new_da_tx: UnboundedSender<BlockInfo>,
        cancellation: CancellationToken,
    ) -> Self::DataAvailabilityWatcher {
        L1WatcherRpc::new(self.l1_provider.clone(), new_da_tx, cancellation)
    }

    async fn init_derivation(&self) -> Result<(L2ForkchoiceState, OnlinePipeline), Self::Error> {
        // Create the caching L1/L2 EL providers for derivation.
        let mut l1_derivation_provider =
            AlloyChainProvider::new(self.l1_provider.clone(), DERIVATION_PROVIDER_CACHE_SIZE);
        let mut l2_derivation_provider = AlloyL2ChainProvider::new(
            self.l2_provider.clone(),
            self.config.clone(),
            DERIVATION_PROVIDER_CACHE_SIZE,
        );

        // Find the starting forkchoice state.
        let starting_forkchoice = find_starting_forkchoice(
            self.config.as_ref(),
            &mut l1_derivation_provider,
            &mut l2_derivation_provider,
        )
        .await?;

        info!(
            target: "rollup_node",
            unsafe = %starting_forkchoice.un_safe.block_info.number,
            safe = %starting_forkchoice.safe.block_info.number,
            finalized = %starting_forkchoice.finalized.block_info.number,
            "Found starting forkchoice state"
        );

        // Start the derivation pipeline's L1 origin 1 channel timeout before the L1 origin of the
        // safe head block.
        let starting_origin_num = starting_forkchoice.safe.l1_origin.number.saturating_sub(
            self.config.channel_timeout(starting_forkchoice.safe.block_info.timestamp),
        );
        let starting_origin =
            l1_derivation_provider.block_info_by_number(starting_origin_num).await?;

        let pipeline = OnlinePipeline::new(
            self.config.clone(),
            starting_forkchoice.safe,
            starting_origin,
            OnlineBlobProvider::init(self.l1_beacon.clone()).await,
            l1_derivation_provider,
            l2_derivation_provider,
        )
        .await?;

        Ok((starting_forkchoice, pipeline))
    }
}

#[async_trait]
impl SequencerNodeService for RollupNode {
    async fn start(&self) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

/// Errors that can occur during the operation of the [RollupNode].
#[derive(Error, Debug)]
pub enum RollupNodeError {
    /// An error occurred while finding the sync starting point.
    #[error(transparent)]
    SyncStart(#[from] SyncStartError),
    /// An error occurred while creating the derivation pipeline.
    #[error(transparent)]
    OnlinePipeline(#[from] PipelineErrorKind),
    /// An error occurred while initializing the derivation pipeline.
    #[error(transparent)]
    AlloyChainProvider(#[from] AlloyChainProviderError),
}
