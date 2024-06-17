//! A program to verify a Ethereum STF in the zkVM using the Kona DB and the Reth Ethereum block
//! executor.

use alloy_consensus::{Header, Sealable};
use alloy_rlp::Decodable;
use kona_client::{
    l1::{DerivationDriver, OracleBlobProvider, OracleL1ChainProvider},
    l2::{OracleL2ChainProvider, TrieDBHintWriter},
    BootInfo, CachingOracle,
};
use kona_derive::types::{L2PayloadAttributes, OP_MAINNET_CONFIG};
use kona_executor::StatelessL2BlockExecutor;
use kona_mpt::{ordered_trie_with_encoder, TrieDB, TrieDBFetcher, TrieDBHinter};
use kona_zkvm::{ZkvmTrieDBFetcher, ZkvmTrieDBHinter};
use reth_chainspec::{ChainSpec, ChainSpecBuilder, ForkCondition, MAINNET};
use reth_evm::execute::BlockExecutorProvider;
use reth_evm_ethereum::{
    execute::{EthBlockExecutor, EthExecutorProvider},
    EthEvmConfig,
};
use reth_primitives::{
    revm::{config::revm_spec, env::fill_tx_env},
    revm_primitives::AnalysisKind,
    Address, Bytecode, Hardfork, Head, TransactionSigned, B256, U256,
};
use reth_revm::{Database, State as RethState};
use revm::{
    db::{states::bundle_state::BundleRetention, State},
    primitives::{
        calc_excess_blob_gas, BlobExcessGasAndPrice, BlockEnv, CfgEnv, CfgEnvWithHandlerCfg,
        EnvWithHandlerCfg, OptimismFields, SpecId, TransactTo, TxEnv,
    },
    Database as RevmDatabase, Evm, StateBuilder,
};
use std::sync::Arc;

pub fn main() {
    let fetcher: ZkvmTrieDBFetcher = sp1_zkvm::io::read();
    let hinter = ZkvmTrieDBHinter {};

    // TODO: grab some test data with a real Ethereum block header

    let raw_header = sp1_zkvm::io::read_vec();
    let parent_header = Header::decode(&mut &raw_header[..]).unwrap();
    let sealed_header = parent_header.clone().seal_slow();
    let trie_db = TrieDB::new(parent_header.state_root, sealed_header, fetcher, hinter);
    let state = State::builder().with_database(trie_db).with_bundle_update().build();

    let evm_config: EthEvmConfig = Default::default();
    let chain_spec = Arc::new(
        ChainSpecBuilder::from(&*MAINNET)
            .shanghai_activated()
            .with_fork(Hardfork::Cancun, ForkCondition::Timestamp(1))
            .build(),
    );

    let provider = EthExecutorProvider::mainnet();
    // let executor = EthBlockExecutor::new(chain_spec, evm_config, state);
}

// struct Wrapper<F: kona_mpt::TrieDBFetcher, H: kona_mpt::TrieDBHinter>(TrieDB<F, H>);

// impl<F: kona_mpt::TrieDBFetcher, H: kona_mpt::TrieDBHinter> Database for Wrapper<F, H> {
//     type Error = ProviderError;
//     fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
//         self.basic(address)
//     }
//     fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
//         self.0.block_hash(number).map_err(|e| ProviderError::UnsupportedProvider)
//     }
//     fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
//         self.0.code_by_hash(code_hash).map_err(|e| ProviderError::UnsupportedProvider)
//     }
//     fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
//         self.storage(address, index)
//     }
// }
