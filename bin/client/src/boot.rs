//! This module contains the [BootInfo] struct, which contains bootstrap information for the `client` program.

use alloy_primitives::B256;
use anyhow::{anyhow, Result};
use kona_preimage::{OracleReader, PreimageKey, PreimageOracleClient};

/// The [BootInfo] struct contains bootstrap information for the `client` program. This information is used to
/// initialize chain derivation as well as verify the integrity of the L2 claim versus the produced L2 output root.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct BootInfo {
    /// The L1 head hash containing all data necessary to derive the `l2_claim` state.
    pub(crate) l1_head: B256,
    /// The starting L2 output root hash.
    pub(crate) starting_l2_output_root: B256,
    /// The claimed L2 output root hash.
    pub(crate) claimed_l2_output_root: B256,
    /// The claimed L2 output root block number.
    pub(crate) claimed_l2_output_root_block_number: u64,
    /// The L2 chain ID.
    pub(crate) l2_chain_id: u64,
    // /// The L2 chain configuration.
    // pub(crate) l2_chain_config: L2ChainConfig,
    // /// The rollup configuration.
    // pub(crate) rollup_config: RollupConfig,
}

/// A [LocalKeyIndex] is a unique identifier for a local preimage key in the `PreimageOracle`. These keys are used to
/// store and retrieve bootstrap information for the `client` program.
#[repr(u8)]
pub(crate) enum LocalKeyIndex {
    L1Head = 1,
    StartingL2OutputRoot = 2,
    ClaimedL2OutputRoot = 3,
    ClaimedL2OutputRootBlockNumber = 4,
    L2ChainId = 5,
    // L2ChainConfig = 6,
    // RollupConfig = 7,
}

impl BootInfo {
    /// Attempts to boot the client program by reading the necessary bootstrap information from the [OracleReader]
    /// passed. If any of the required keys are missing or malformatted, an error is returned.
    pub(crate) fn try_boot(oracle: &OracleReader) -> Result<Self> {
        let l1_head: B256 = oracle
            .get(PreimageKey::new_local(LocalKeyIndex::L1Head as u64))?
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("Failed to convert L1 head hash slice to `B256`"))?;
        let starting_l2_output_root: B256 = oracle
            .get(PreimageKey::new_local(
                LocalKeyIndex::StartingL2OutputRoot as u64,
            ))?
            .as_slice()
            .try_into()
            .map_err(|_| {
                anyhow!("Failed to convert starting L2 output root hash slice to `B256`")
            })?;
        let claimed_l2_output_root: B256 = oracle
            .get(PreimageKey::new_local(
                LocalKeyIndex::ClaimedL2OutputRoot as u64,
            ))?
            .as_slice()
            .try_into()
            .map_err(|_| {
                anyhow!("Failed to convert claimed L2 output root hash slice to `B256`")
            })?;
        let claimed_l2_output_root_block_number: u64 = u64::from_be_bytes(
            oracle
                .get(PreimageKey::new_local(
                    LocalKeyIndex::ClaimedL2OutputRootBlockNumber as u64,
                ))?
                .as_slice()
                .try_into()
                .map_err(|_| {
                    anyhow!("Failed to convert claimed L2 output root block number slice to `u64`")
                })?,
        );
        let l2_chain_id: u64 = u64::from_be_bytes(
            oracle
                .get(PreimageKey::new_local(LocalKeyIndex::L2ChainId as u64))?
                .as_slice()
                .try_into()
                .map_err(|_| anyhow!("Failed to convert L2 chain ID slice to `u64`"))?,
        );

        Ok(Self {
            l1_head,
            starting_l2_output_root,
            claimed_l2_output_root,
            claimed_l2_output_root_block_number,
            l2_chain_id,
        })
    }
}
