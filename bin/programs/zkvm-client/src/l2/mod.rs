//! Contains the L2-specifc contstructs of the client program.

mod chain_provider;
// pub use chain_provider::OracleL2ChainProvider;
use kona_client::l1::OracleL1ChainProvider;
use kona_preimage::CommsClient;
mod trie_hinter;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use alloy_consensus::{Header, Receipt, ReceiptEnvelope, TxEnvelope};
use alloy_eips::eip2718::Decodable2718;
use alloy_primitives::{Bytes, B256};
use alloy_rlp::Decodable;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kona_derive::traits::ChainProvider;
use kona_mpt::{OrderedListWalker, TrieDBFetcher};
use kona_preimage::{HintWriterClient, PreimageKey, PreimageKeyType, PreimageOracleClient};
use kona_primitives::BlockInfo;

// #[allow(unused_imports)]
// pub use trie_hinter::TrieDBHintWriter;

// TODO: add "WithOracle"
pub struct WrappingOracleL1ChainProvider<P> {
    pub l1_provider: P,
}

#[async_trait]
impl<P: ChainProvider + Send + Sync> ChainProvider for WrappingOracleL1ChainProvider<P> {
    async fn header_by_hash(&mut self, hash: B256) -> Result<Header> {
        todo!();
        // This is the only one we have to add checks to.
        // let oracle = self.l1_provider.oracle;

        // Fetch the header RLP from the oracle.
        // let header_rlp = oracle.get(PreimageKey::new(*hash, PreimageKeyType::Keccak256)).await?;

        // TODO: do the keccak check.

        // Decode the header RLP into a Header.
        // Header::decode(&mut header_rlp.as_slice())
        //     .map_err(|e| anyhow!("Failed to decode header RLP: {e}"))
    }

    async fn block_info_by_number(&mut self, block_number: u64) -> Result<BlockInfo> {
        self.l1_provider.block_info_by_number(block_number).await
    }

    async fn receipts_by_hash(&mut self, hash: B256) -> Result<Vec<Receipt>> {
        self.l1_provider.receipts_by_hash(hash).await
    }

    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>)> {
        self.l1_provider.block_info_and_transactions_by_hash(hash).await
    }
}

impl<P: TrieDBFetcher + Send + Sync> TrieDBFetcher for WrappingOracleL1ChainProvider<P> {
    fn trie_node_preimage(&self, key: B256) -> Result<Bytes> {
        // On L1, trie node preimages are stored as keccak preimage types in the oracle. We assume
        // that a hint for these preimages has already been sent, prior to this call.
        let result = self.l1_provider.trie_node_preimage(key)?;
        // TODO: check keccak
        Ok(result)
    }

    fn bytecode_by_hash(&self, _: B256) -> Result<Bytes> {
        unimplemented!("TrieDBFetcher::bytecode_by_hash unimplemented for OracleL1ChainProvider")
    }

    fn header_by_hash(&self, _: B256) -> Result<Header> {
        unimplemented!("TrieDBFetcher::header_by_hash unimplemented for OracleL1ChainProvider")
    }
}
