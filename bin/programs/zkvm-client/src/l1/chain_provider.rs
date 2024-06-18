//! Contains the concrete implementation of the [ChainProvider] trait for the client program.

use crate::{BootInfo, InMemoryOracle};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use alloy_consensus::{Header, Receipt, ReceiptEnvelope, TxEnvelope};
use alloy_eips::eip2718::Decodable2718;
use alloy_primitives::{Bytes, B256, keccak256};
use alloy_rlp::Decodable;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kona_derive::traits::ChainProvider;
use kona_mpt::{OrderedListWalker, TrieDBFetcher};
use kona_preimage::{PreimageKey, PreimageKeyType, PreimageOracleClient};
use kona_primitives::BlockInfo;

/// The oracle-backed L1 chain provider for the client program.
#[derive(Debug, Clone)]
pub struct OracleL1ChainProvider {
    /// The boot information
    boot_info: Arc<BootInfo>,
    /// The preimage oracle client.
    oracle: Arc<InMemoryOracle>,
}

impl OracleL1ChainProvider {
    /// Creates a new [OracleL1ChainProvider] with the given boot information and oracle client.
    pub fn new(boot_info: Arc<BootInfo>, oracle: Arc<InMemoryOracle>) -> Self {
        Self { boot_info, oracle }
    }
}

#[async_trait]
impl ChainProvider for OracleL1ChainProvider {
    async fn header_by_hash(&mut self, hash: B256) -> Result<Header> {
        // Fetch the header RLP from the oracle.
        let header_rlp =
            self.oracle.get(PreimageKey::new(*hash, PreimageKeyType::Keccak256)).await?;

        // ZKVM Constraint: keccak(header_rlp) = hash
        assert_eq!(keccak256(&header_rlp), hash, "header_by_hash - zkvm constraint failed");

        // Decode the header RLP into a Header.
        Header::decode(&mut header_rlp.as_slice())
            .map_err(|e| anyhow!("Failed to decode header RLP: {e}"))
    }

    async fn block_info_by_number(&mut self, block_number: u64) -> Result<BlockInfo> {
        // Fetch the starting block header.
        let mut header = self.header_by_hash(self.boot_info.l1_head).await?;

        // Check if the block number is in range. If not, we can fail early.
        if block_number > header.number {
            anyhow::bail!("Block number past L1 head.");
        }

        // Walk back the block headers to the desired block number.
        while header.number > block_number {
            header = self.header_by_hash(header.parent_hash).await?;
        }

        Ok(BlockInfo {
            hash: header.hash_slow(),
            number: header.number,
            parent_hash: header.parent_hash,
            timestamp: header.timestamp,
        })
    }

    async fn receipts_by_hash(&mut self, hash: B256) -> Result<Vec<Receipt>> {
        // Fetch the block header to find the receipts root.
        let header = self.header_by_hash(hash).await?;

        // Walk through the receipts trie in the header to verify them.
        let trie_walker = OrderedListWalker::try_new_hydrated(header.receipts_root, self)?;

        // Decode the receipts within the transactions trie.
        let receipts = trie_walker
            .into_iter()
            .map(|(_, rlp)| {
                let envelope = ReceiptEnvelope::decode_2718(&mut rlp.as_ref())
                    .map_err(|e| anyhow!("Failed to decode ReceiptEnvelope RLP: {e}"))?;
                Ok(envelope.as_receipt().expect("Infalliable").clone())
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(receipts)
    }

    async fn block_info_and_transactions_by_hash(
        &mut self,
        hash: B256,
    ) -> Result<(BlockInfo, Vec<TxEnvelope>)> {
        // Fetch the block header to construct the block info.
        let header = self.header_by_hash(hash).await?;
        let block_info = BlockInfo {
            hash,
            number: header.number,
            parent_hash: header.parent_hash,
            timestamp: header.timestamp,
        };

        // Walk through the transactions trie in the header to verify them.
        let trie_walker = OrderedListWalker::try_new_hydrated(header.transactions_root, self)?;

        // Decode the transactions within the transactions trie.
        let transactions = trie_walker
            .into_iter()
            .map(|(_, rlp)| {
                TxEnvelope::decode_2718(&mut rlp.as_ref())
                    .map_err(|e| anyhow!("Failed to decode TxEnvelope RLP: {e}"))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((block_info, transactions))
    }
}

impl TrieDBFetcher for OracleL1ChainProvider {
    fn trie_node_preimage(&self, key: B256) -> Result<Bytes> {
        // On L1, trie node preimages are stored as keccak preimage types in the oracle. We assume
        // that a hint for these preimages has already been sent, prior to this call.
        kona_common::block_on(async move {
            let preimage = self.oracle
                .get(PreimageKey::new(*key, PreimageKeyType::Keccak256))
                .await
                .map(Into::into)
                .unwrap();

            // ZKVM Constraint: keccak(node preimage) = hash
            assert_eq!(keccak256(&preimage), key, "trie_node_preimage - zkvm constraint failed");

            Ok(preimage)
        })
    }

    fn bytecode_by_hash(&self, _: B256) -> Result<Bytes> {
        unimplemented!("TrieDBFetcher::bytecode_by_hash unimplemented for OracleL1ChainProvider")
    }

    fn header_by_hash(&self, _: B256) -> Result<Header> {
        unimplemented!("TrieDBFetcher::header_by_hash unimplemented for OracleL1ChainProvider")
    }
}
