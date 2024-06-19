// //! This module contains the prologue phase of the client program, pulling in the boot
// information //! through the `PreimageOracle` ABI as local keys.

// use alloy_primitives::{B256, U256};
// use anyhow::{anyhow, Result};
// use crate::Oracle;
// use kona_preimage::{PreimageKey, PreimageOracleClient};
// use kona_primitives::{RollupConfig, OP_MAINNET_CONFIG};
// use serde::{Serialize, Deserialize};

// /// The local key ident for the L1 head hash.
// pub const L1_HEAD_KEY: U256 = U256::from_be_slice(&[1]);

// /// The local key ident for the L2 output root.
// pub const L2_OUTPUT_ROOT_KEY: U256 = U256::from_be_slice(&[2]);

// /// The local key ident for the L2 output root claim.
// pub const L2_CLAIM_KEY: U256 = U256::from_be_slice(&[3]);

// /// The local key ident for the L2 claim block number.
// pub const L2_CLAIM_BLOCK_NUMBER_KEY: U256 = U256::from_be_slice(&[4]);

// /// The local key ident for the L2 chain ID.
// pub const L2_CHAIN_ID_KEY: U256 = U256::from_be_slice(&[5]);

// /// The local key ident for the L2 rollup config.
// #[allow(dead_code)]
// pub const L2_ROLLUP_CONFIG_KEY: U256 = U256::from_be_slice(&[6]);

// /// The boot information for the client program.
// ///
// /// **Verified inputs:**
// /// - `l1_head`: The L1 head hash containing the safe L2 chain data that may reproduce the L2
// head ///   hash.
// /// - `l2_output_root`: The latest finalized L2 output root.
// /// - `chain_id`: The L2 chain ID.
// ///
// /// **User submitted inputs:**
// /// - `l2_claim`: The L2 output root claim.
// /// - `l2_claim_block`: The L2 claim block number.
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct BootInfo {
//     /// The L1 head hash containing the safe L2 chain data that may reproduce the L2 head hash.
//     pub l1_head: B256,
//     /// The latest finalized L2 output root.
//     pub l2_output_root: B256,
//     /// The L2 output root claim.
//     pub l2_claim: B256,
//     /// The L2 claim block number.
//     pub l2_claim_block: u64,
//     /// The L2 chain ID.
//     pub chain_id: u64,
//     /// The rollup config for the L2 chain.
//     pub rollup_config: RollupConfig,
// }

// impl BootInfo {
//     /// Load the boot information from the preimage oracle.
//     ///
//     /// ## Takes
//     /// - `oracle`: The preimage oracle reader.
//     ///
//     /// ## Returns
//     /// - `Ok(BootInfo)`: The boot information.
//     /// - `Err(_)`: Failed to load the boot information.
//     pub async fn load(oracle: &Oracle) -> Result<Self> {
//         if let Oracle::Caching(oracle) = oracle {
//             let mut l1_head: B256 = B256::ZERO;
//             oracle.get_exact(PreimageKey::new_local(L1_HEAD_KEY.to()), l1_head.as_mut()).await?;

//             let mut l2_output_root: B256 = B256::ZERO;
//             oracle
//                 .get_exact(PreimageKey::new_local(L2_OUTPUT_ROOT_KEY.to()),
// l2_output_root.as_mut())                 .await?;

//             let mut l2_claim: B256 = B256::ZERO;
//             oracle.get_exact(PreimageKey::new_local(L2_CLAIM_KEY.to()),
// l2_claim.as_mut()).await?;

//             let l2_claim_block = u64::from_be_bytes(
//                 oracle
//                     .get(PreimageKey::new_local(L2_CLAIM_BLOCK_NUMBER_KEY.to()))
//                     .await?
//                     .try_into()
//                     .map_err(|_| anyhow!("Failed to convert L2 claim block number to u64"))?,
//             );
//             let chain_id = u64::from_be_bytes(
//                 oracle
//                     .get(PreimageKey::new_local(L2_CHAIN_ID_KEY.to()))
//                     .await?
//                     .try_into()
//                     .map_err(|_| anyhow!("Failed to convert L2 chain ID to u64"))?,
//             );
//             let rollup_config = rollup_config_from_chain_id(chain_id)?;

//             Ok(Self { l1_head, l2_output_root, l2_claim, l2_claim_block, chain_id, rollup_config
// })         } else {
//             anyhow::bail!("load can only be called with caching oracle.")
//         }

//     }
// }

// /// Returns the rollup config for the given chain ID.
// fn rollup_config_from_chain_id(chain_id: u64) -> Result<RollupConfig> {
//     match chain_id {
//         10 => Ok(OP_MAINNET_CONFIG),
//         _ => anyhow::bail!("Unsupported chain ID: {}", chain_id),
//     }
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct BootInfoWithoutRollupConfig {
//     pub l1_head: B256,
//     pub l2_output_root: B256,
//     pub l2_claim: B256,
//     pub l2_claim_block: u64,
//     pub chain_id: u64,
// }

// impl From<BootInfoWithoutRollupConfig> for BootInfo {
//     fn from(boot_info_without_rollup_config: BootInfoWithoutRollupConfig) -> Self {
//         let BootInfoWithoutRollupConfig { l1_head, l2_output_root, l2_claim, l2_claim_block,
// chain_id } = boot_info_without_rollup_config;         let rollup_config =
// rollup_config_from_chain_id(chain_id).unwrap();

//         Self { l1_head, l2_output_root, l2_claim, l2_claim_block, chain_id, rollup_config }
//     }
// }
