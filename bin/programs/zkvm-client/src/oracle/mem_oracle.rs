use alloc::{boxed::Box, vec::Vec};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use hashbrown::HashMap;
use kona_preimage::{HintWriterClient, PreimageKey, PreimageOracleClient};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InMemoryOracle {
    cache: HashMap<PreimageKey, Vec<u8>>,
}

impl InMemoryOracle {
    pub fn from_raw_bytes(input: Vec<u8>) -> Self {
        Self {
            // Z-TODO: Use more efficient library for deserialization.
            // https://github.com/rkyv/rkyv
            cache: bincode::deserialize(&input).unwrap(),
        }
    }
}

#[async_trait]
impl PreimageOracleClient for InMemoryOracle {
    async fn get(&self, key: PreimageKey) -> Result<Vec<u8>> {
        self.cache.get(&key).cloned().ok_or_else(|| anyhow!("Key not found in cache"))
    }

    async fn get_exact(&self, key: PreimageKey, buf: &mut [u8]) -> Result<()> {
        let value = self.cache.get(&key).ok_or_else(|| anyhow!("Key not found in cache"))?;
        buf.copy_from_slice(value.as_slice());
        Ok(())
    }
}

#[async_trait]
impl HintWriterClient for InMemoryOracle {
    async fn write(&self, hint: &str) -> Result<()> {
        Ok(())
    }
}
