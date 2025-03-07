//! An Engine API Client.

use alloy_eips::eip1898::BlockNumberOrTag;
use alloy_network::AnyNetwork;
use alloy_primitives::{B256, Bytes};
use alloy_provider::RootProvider;
use alloy_rpc_client::RpcClient;
use alloy_rpc_types_engine::{
    ForkchoiceState, ForkchoiceUpdated, JwtSecret, PayloadId, PayloadStatus,
};
use alloy_transport_http::{
    AuthLayer, AuthService, Http, HyperClient,
    hyper_util::{
        client::legacy::{Client, connect::HttpConnector},
        rt::TokioExecutor,
    },
};
use anyhow::Result;
use http_body_util::Full;
use op_alloy_provider::ext::engine::OpEngineApi;
use op_alloy_rpc_types_engine::OpPayloadAttributes;
use std::sync::Arc;
use tower::ServiceBuilder;
use url::Url;

use kona_genesis::RollupConfig;
use kona_protocol::L2BlockInfo;
use kona_providers_alloy::AlloyL2ChainProvider;

/// A Hyper HTTP client with a JWT authentication layer.
type HyperAuthClient<B = Full<Bytes>> = HyperClient<B, AuthService<Client<HttpConnector, B>>>;

/// An external engine api client
#[derive(Debug, Clone)]
pub struct EngineClient {
    /// The L2 engine provider.
    engine: RootProvider<AnyNetwork>,
    /// The L2 chain provider.
    #[allow(unused)]
    rpc: AlloyL2ChainProvider,
    /// The [RollupConfig] for the chain used to timestamp which version of the engine api to use.
    #[allow(unused)]
    cfg: Arc<RollupConfig>,
}

impl EngineClient {
    /// Creates a new [`EngineClient`] from the provided [Url] and [JwtSecret].
    pub fn new_http(engine: Url, rpc: Url, cfg: Arc<RollupConfig>, jwt: JwtSecret) -> Self {
        let hyper_client = Client::builder(TokioExecutor::new()).build_http::<Full<Bytes>>();

        let auth_layer = AuthLayer::new(jwt);
        let service = ServiceBuilder::new().layer(auth_layer).service(hyper_client);

        let layer_transport = HyperClient::with_service(service);
        let http_hyper = Http::with_client(layer_transport, engine);
        let rpc_client = RpcClient::new(http_hyper, true);
        let engine = RootProvider::<AnyNetwork>::new(rpc_client);

        let rpc = RootProvider::new_http(rpc);
        let rpc = AlloyL2ChainProvider::new(rpc, cfg.clone());
        Self { engine, rpc, cfg }
    }

    /// Attempts to update the engine forkchoice state with the given attributes.
    pub async fn try_forkchoice_update(
        &self,
        forkchoice: ForkchoiceState,
        attributes: Option<OpPayloadAttributes>,
    ) -> Result<ForkchoiceUpdated> {
        let forkchoice = <RootProvider<AnyNetwork> as OpEngineApi<
            AnyNetwork,
            Http<HyperAuthClient>,
        >>::fork_choice_updated_v2(&self.engine, forkchoice, attributes)
        .await?;
        Ok(forkchoice)
    }

    /// Gets the payload by the given payload id.
    pub async fn get_payload<T>(&self, _payload_id: PayloadId) -> Result<T> {
        unimplemented!("get_payload_v3 not implemented")
    }

    /// Returns the status of the given payload.
    pub async fn new_payload<P>(
        &self,
        _payload: P,
        _parent_beacon_block_root: B256,
    ) -> Result<PayloadStatus> {
        unimplemented!("new_payload_v3 not implemented")
    }

    /// Fetches the [L2BlockInfo] by [BlockNumberOrTag].
    pub async fn l2_block_info_by_label(
        &mut self,
        _numtag: BlockNumberOrTag,
    ) -> Result<L2BlockInfo> {
        unimplemented!("L2BlockInfo by label not implemented")
    }
}

impl std::ops::Deref for EngineClient {
    type Target = RootProvider<AnyNetwork>;

    fn deref(&self) -> &Self::Target {
        &self.engine
    }
}
