use alloy_consensus::Header;
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use alloy_rlp::Decodable;
use anyhow::{anyhow, Result};
use kona_mpt::{TrieDBFetcher, TrieDBHinter};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;

/// A [TrieDBFetcher] for usage in zkVM programs.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ZkvmTrieDBFetcher {
    preimages: HashMap<B256, Bytes>,
}

impl ZkvmTrieDBFetcher {
    #[cfg(not(target_os = "zkvm"))]
    /// Constructs a new [ZkvmTrieDBFetcher] from a testdata file. Only available in the host
    /// environment.
    pub fn from_file(file_name: &str) -> Self {
        let preimages = serde_json::from_str::<HashMap<B256, Bytes>>(
            &std::fs::read_to_string(file_name).unwrap(),
        )
        .unwrap();
        Self { preimages }
    }

    /// Verifies that all preimages in the [ZkvmTrieDBFetcher] are correct.
    pub fn verify(&self) {
        for (key, value) in self.preimages.iter() {
            assert_eq!(keccak256(value), *key);
        }
    }
}

impl TrieDBFetcher for ZkvmTrieDBFetcher {
    fn trie_node_preimage(&self, key: B256) -> Result<Bytes> {
        self.preimages
            .get(&key)
            .cloned()
            .ok_or_else(|| anyhow!("Preimage not found for key: {}", key))
    }

    fn bytecode_by_hash(&self, code_hash: B256) -> Result<Bytes> {
        self.preimages
            .get(&code_hash)
            .cloned()
            .ok_or_else(|| anyhow!("Bytecode not found for hash: {}", code_hash))
    }

    fn header_by_hash(&self, hash: B256) -> Result<Header> {
        let encoded_header = self
            .preimages
            .get(&hash)
            .ok_or_else(|| anyhow!("Header not found for hash: {}", hash))?;
        // TODO: there might be an optimization where we can cache the header decoding if we are
        // decoding the same header many times.
        Header::decode(&mut encoded_header.as_ref()).map_err(|e| anyhow!(e))
    }
}

/// A [TrieDBHinter] for usage in zkVM programs.
pub struct ZkvmTrieDBHinter;

impl ZkvmTrieDBHinter {
    pub fn new() -> Self {
        Self
    }
}

impl TrieDBHinter for ZkvmTrieDBHinter {
    fn hint_trie_node(&self, _hash: B256) -> Result<()> {
        // TODO: implement this method.
        Ok(())
    }

    fn hint_account_proof(&self, _address: Address, _block_number: u64) -> Result<()> {
        // TODO: implement this method.
        Ok(())
    }

    fn hint_storage_proof(&self, _address: Address, _slot: U256, _block_number: u64) -> Result<()> {
        // TODO: implement this method.
        Ok(())
    }
}
