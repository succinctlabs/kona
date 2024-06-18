//! Contains the host <-> client communication utilities.

use kona_common::FileDescriptor;
use kona_preimage::{HintWriter, OracleReader, PipeHandle, PreimageOracleClient, PreimageKey};
use async_trait::async_trait;
use alloc::{boxed::Box, vec::Vec};
use anyhow::Result;

mod mem_oracle;
pub use mem_oracle::InMemoryOracle;

mod caching_oracle;
pub use caching_oracle::CachingOracle;

/// The global preimage oracle reader pipe.
static ORACLE_READER_PIPE: PipeHandle =
    PipeHandle::new(FileDescriptor::PreimageRead, FileDescriptor::PreimageWrite);

/// The global hint writer pipe.
static HINT_WRITER_PIPE: PipeHandle =
    PipeHandle::new(FileDescriptor::HintRead, FileDescriptor::HintWrite);

/// The global preimage oracle reader.
pub static ORACLE_READER: OracleReader = OracleReader::new(ORACLE_READER_PIPE);

/// The global hint writer.
pub static HINT_WRITER: HintWriter = HintWriter::new(HINT_WRITER_PIPE);

#[derive(Debug, Clone)]
pub enum Oracle {
    InMemory(InMemoryOracle),
    Caching(CachingOracle),
}

impl Oracle {
    pub fn new_in_memory(input: Vec<u8>) -> Self {
        Self::InMemory(InMemoryOracle::from_raw_bytes(input))
    }

    pub fn new_caching(cache_size: usize) -> Self {
        Self::Caching(CachingOracle::new(cache_size))
    }
}


#[async_trait]
impl PreimageOracleClient for Oracle {
    async fn get(&self, key: PreimageKey) -> Result<Vec<u8>> {
        match self {
            Self::InMemory(oracle) => oracle.get(key).await,
            Self::Caching(oracle) => oracle.get(key).await,
        }
    }

    async fn get_exact(&self, key: PreimageKey, buf: &mut [u8]) -> Result<()> {
        match self {
            Self::InMemory(oracle) => oracle.get_exact(key, buf).await,
            Self::Caching(oracle) => oracle.get_exact(key, buf).await,
        }
    }
}
