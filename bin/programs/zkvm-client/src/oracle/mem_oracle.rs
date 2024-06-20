use alloc::{boxed::Box, vec::Vec};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use kona_preimage::{PreimageKey, PreimageOracleClient};
// use hashbrown::HashMap;
// use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use rkyv::{Archive, Serialize, Deserialize, Infallible};

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
pub struct InMemoryOracle {
    cache: HashMap<PreimageKey, Vec<u8>>,
}

impl InMemoryOracle {
    pub fn from_raw_bytes(input: Vec<u8>) -> Self {
        let archived = unsafe { rkyv::archived_root::<HashMap<PreimageKey, Vec<u8>>>(&input) };
        let deserialized: HashMap<PreimageKey, Vec<u8>> = archived.deserialize(&mut Infallible).unwrap();

        Self {
            cache: deserialized,
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
