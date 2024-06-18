//! This module contains the prologue phase of the client program, pulling in the boot information
//! through the `PreimageOracle` ABI as local keys.

use alloy_primitives::B256;
use anyhow::Result;
use kona_primitives::{RollupConfig, OP_MAINNET_CONFIG};

/// The boot information for the client program.
///
/// **Verified inputs:**
/// - `l1_head`: The L1 head hash containing the safe L2 chain data that may reproduce the L2 head
///   hash.
/// - `l2_output_root`: The latest finalized L2 output root.
/// - `chain_id`: The L2 chain ID.
///
/// **User submitted inputs:**
/// - `l2_claim`: The L2 output root claim.
/// - `l2_claim_block`: The L2 claim block number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootInfo {
    /// The L1 head hash containing the safe L2 chain data that may reproduce the L2 head hash.
    pub l1_head: B256,
    /// The latest finalized L2 output root.
    pub l2_output_root: B256,
    /// The L2 output root claim.
    pub l2_claim: B256,
    /// The L2 claim block number.
    pub l2_claim_block: u64,
    /// The L2 chain ID.
    pub chain_id: u64,
    /// The rollup config for the L2 chain.
    pub rollup_config: RollupConfig,
}

impl BootInfo {
    pub fn new(l1_head: B256, l2_output_root: B256, l2_claim: B256, l2_claim_block: u64, chain_id: u64) -> Self {
        let rollup_config = rollup_config_from_chain_id(chain_id)?;

        Self { l1_head, l2_output_root, l2_claim, l2_claim_block, chain_id, rollup_config }
    }
}

/// Returns the rollup config for the given chain ID.
fn rollup_config_from_chain_id(chain_id: u64) -> Result<RollupConfig> {
    match chain_id {
        10 => Ok(OP_MAINNET_CONFIG),
        _ => anyhow::bail!("Unsupported chain ID: {}", chain_id),
    }
}
