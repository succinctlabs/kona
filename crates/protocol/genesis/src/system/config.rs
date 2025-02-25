//! Contains the [`SystemConfig`] type.

use alloy_consensus::{Eip658Value, Receipt};
use alloy_primitives::{Address, Log, B64, U256};

use crate::{
    RollupConfig, SystemConfigLog, SystemConfigUpdateError, SystemConfigUpdateKind,
    CONFIG_UPDATE_TOPIC,
};

/// System configuration.
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SystemConfig {
    /// Batcher address
    #[cfg_attr(feature = "serde", serde(rename = "batchSubmitter", alias = "batchSubmitter"))]
    pub batch_submitter: Address,
    /// Fee overhead value
    pub overhead: U256,
    /// Fee scalar value
    pub scalar: U256,
    /// Gas limit value
    pub gas_limit: u64,
    /// Base fee scalar value
    pub base_fee_scalar: Option<u64>,
    /// Blob base fee scalar value
    pub blob_base_fee_scalar: Option<u64>,
    /// EIP-1559 denominator
    pub eip1559_denominator: Option<u32>,
    /// EIP-1559 elasticity
    pub eip1559_elasticity: Option<u32>,
    /// The operator fee scalar (isthmus hardfork)
    pub operator_fee_scalar: Option<u32>,
    /// The operator fee constant (isthmus hardfork)
    pub operator_fee_constant: Option<u64>,
}

/// Custom EIP-1559 parameter decoding is needed here for holocene encoding.
///
/// This is used by the Optimism monorepo [here][here].
///
/// [here]: https://github.com/ethereum-optimism/optimism/blob/cf28bffc7d880292794f53bb76bfc4df7898307b/op-service/eth/types.go#L519

#[cfg(feature = "serde")]
impl<'a> serde::Deserialize<'a> for SystemConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        // An alias struct that is identical to `SystemConfig`.
        // We use the alias to decode the eip1559 params as their u32 values.
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[serde(deny_unknown_fields)]
        struct SystemConfigAlias {
            #[serde(rename = "batchSubmitter", alias = "batchSubmitterAddr")]
            batch_submitter: Address,
            overhead: U256,
            scalar: U256,
            gas_limit: u64,
            base_fee_scalar: Option<u64>,
            blob_base_fee_scalar: Option<u64>,
            eip1559_params: Option<B64>,
            eip1559_denominator: Option<u32>,
            eip1559_elasticity: Option<u32>,
            operator_fee_scalar: Option<u32>,
            operator_fee_constant: Option<u64>,
        }

        let mut alias = SystemConfigAlias::deserialize(deserializer)?;
        if let Some(params) = alias.eip1559_params {
            alias.eip1559_denominator =
                Some(u32::from_be_bytes(params.as_slice().get(0..4).unwrap().try_into().unwrap()));
            alias.eip1559_elasticity =
                Some(u32::from_be_bytes(params.as_slice().get(4..8).unwrap().try_into().unwrap()));
        }

        Ok(Self {
            batch_submitter: alias.batch_submitter,
            overhead: alias.overhead,
            scalar: alias.scalar,
            gas_limit: alias.gas_limit,
            base_fee_scalar: alias.base_fee_scalar,
            blob_base_fee_scalar: alias.blob_base_fee_scalar,
            eip1559_denominator: alias.eip1559_denominator,
            eip1559_elasticity: alias.eip1559_elasticity,
            operator_fee_scalar: alias.operator_fee_scalar,
            operator_fee_constant: alias.operator_fee_constant,
        })
    }
}

impl SystemConfig {
    /// Filters all L1 receipts to find config updates and applies the config updates.
    pub fn update_with_receipts(
        &mut self,
        receipts: &[Receipt],
        l1_system_config_address: Address,
        ecotone_active: bool,
    ) -> Result<(), SystemConfigUpdateError> {
        for receipt in receipts {
            if Eip658Value::Eip658(false) == receipt.status {
                continue;
            }

            receipt.logs.iter().try_for_each(|log| {
                let topics = log.topics();
                if log.address == l1_system_config_address
                    && !topics.is_empty()
                    && topics[0] == CONFIG_UPDATE_TOPIC
                {
                    // Safety: Error is bubbled up by the trailing `?`
                    self.process_config_update_log(log, ecotone_active)?;
                }
                Ok::<(), SystemConfigUpdateError>(())
            })?;
        }
        Ok(())
    }

    /// Returns the eip1559 parameters from a [SystemConfig] encoded as a [B64].
    pub fn eip_1559_params(
        &self,
        rollup_config: &RollupConfig,
        parent_timestamp: u64,
        next_timestamp: u64,
    ) -> Option<B64> {
        let is_holocene = rollup_config.is_holocene_active(next_timestamp);

        // For the first holocene block, a zero'd out B64 is returned to signal the
        // execution layer to use the canyon base fee parameters. Else, the system
        // config's eip1559 parameters are encoded as a B64.
        if is_holocene && !rollup_config.is_holocene_active(parent_timestamp) {
            Some(B64::ZERO)
        } else {
            is_holocene.then_some(B64::from_slice(
                &[
                    self.eip1559_denominator.unwrap_or_default().to_be_bytes(),
                    self.eip1559_elasticity.unwrap_or_default().to_be_bytes(),
                ]
                .concat(),
            ))
        }
    }

    /// Decodes an EVM log entry emitted by the system config contract and applies it as a
    /// [SystemConfig] change.
    ///
    /// Parse log data for:
    ///
    /// ```text
    /// event ConfigUpdate(
    ///    uint256 indexed version,
    ///    UpdateType indexed updateType,
    ///    bytes data
    /// );
    /// ```
    fn process_config_update_log(
        &mut self,
        log: &Log,
        ecotone_active: bool,
    ) -> Result<SystemConfigUpdateKind, SystemConfigUpdateError> {
        // Construct the system config log from the log.
        let log = SystemConfigLog::new(log.clone(), ecotone_active);

        // Construct the update type from the log.
        let update = log.build()?;

        // Apply the update to the system config.
        update.apply(self);

        // Return the update type.
        Ok(update.kind())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::CONFIG_UPDATE_EVENT_VERSION_0;
    use alloc::vec;
    use alloy_primitives::{address, b256, hex, LogData, B256};

    #[test]
    #[cfg(feature = "serde")]
    fn test_system_config_alias() {
        let sc_str: &'static str = r#"{
          "batchSubmitter": "0x6887246668a3b87F54DeB3b94Ba47a6f63F32985",
          "overhead": "0x00000000000000000000000000000000000000000000000000000000000000bc",
          "scalar": "0x00000000000000000000000000000000000000000000000000000000000a6fe0",
          "gasLimit": 30000000
        }"#;
        let system_config: SystemConfig = serde_json::from_str(sc_str).unwrap();
        assert_eq!(
            system_config,
            SystemConfig {
                batch_submitter: address!("6887246668a3b87F54DeB3b94Ba47a6f63F32985"),
                overhead: U256::from(0xbc),
                scalar: U256::from(0xa6fe0),
                gas_limit: 30000000,
                ..Default::default()
            }
        );
    }

    #[test]
    #[cfg(feature = "arbitrary")]
    fn test_arbitrary_system_config() {
        use arbitrary::Arbitrary;
        use rand::Rng;
        let mut bytes = [0u8; 1024];
        rand::rng().fill(bytes.as_mut_slice());
        SystemConfig::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }

    #[test]
    fn test_eip_1559_params_from_system_config_none() {
        let rollup_config = RollupConfig::default();
        let sys_config = SystemConfig::default();
        assert_eq!(sys_config.eip_1559_params(&rollup_config, 0, 0), None);
    }

    #[test]
    fn test_eip_1559_params_from_system_config_some() {
        let rollup_config = RollupConfig { holocene_time: Some(0), ..Default::default() };
        let sys_config = SystemConfig {
            eip1559_denominator: Some(1),
            eip1559_elasticity: None,
            ..Default::default()
        };
        let expected = Some(B64::from_slice(&[1u32.to_be_bytes(), 0u32.to_be_bytes()].concat()));
        assert_eq!(sys_config.eip_1559_params(&rollup_config, 0, 0), expected);
    }

    #[test]
    fn test_eip_1559_params_from_system_config() {
        let rollup_config = RollupConfig { holocene_time: Some(0), ..Default::default() };
        let sys_config = SystemConfig {
            eip1559_denominator: Some(1),
            eip1559_elasticity: Some(2),
            ..Default::default()
        };
        let expected = Some(B64::from_slice(&[1u32.to_be_bytes(), 2u32.to_be_bytes()].concat()));
        assert_eq!(sys_config.eip_1559_params(&rollup_config, 0, 0), expected);
    }

    #[test]
    fn test_default_eip_1559_params_from_system_config() {
        let rollup_config = RollupConfig { holocene_time: Some(0), ..Default::default() };
        let sys_config = SystemConfig {
            eip1559_denominator: None,
            eip1559_elasticity: None,
            ..Default::default()
        };
        let expected = Some(B64::ZERO);
        assert_eq!(sys_config.eip_1559_params(&rollup_config, 0, 0), expected);
    }

    #[test]
    fn test_default_eip_1559_params_from_system_config_pre_holocene() {
        let rollup_config = RollupConfig::default();
        let sys_config = SystemConfig {
            eip1559_denominator: Some(1),
            eip1559_elasticity: Some(2),
            ..Default::default()
        };
        assert_eq!(sys_config.eip_1559_params(&rollup_config, 0, 0), None);
    }

    #[test]
    fn test_default_eip_1559_params_first_block_holocene() {
        let rollup_config = RollupConfig { holocene_time: Some(2), ..Default::default() };
        let sys_config = SystemConfig {
            eip1559_denominator: Some(1),
            eip1559_elasticity: Some(2),
            ..Default::default()
        };
        assert_eq!(sys_config.eip_1559_params(&rollup_config, 0, 2), Some(B64::ZERO));
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_system_config_serde() {
        let sc_str = r#"{
          "batcherAddr": "0x6887246668a3b87F54DeB3b94Ba47a6f63F32985",
          "overhead": "0x00000000000000000000000000000000000000000000000000000000000000bc",
          "scalar": "0x00000000000000000000000000000000000000000000000000000000000a6fe0",
          "gasLimit": 30000000
        }"#;
        let system_config: SystemConfig = serde_json::from_str(sc_str).unwrap();
        assert_eq!(
            system_config.batcher_address,
            address!("6887246668a3b87F54DeB3b94Ba47a6f63F32985")
        );
        assert_eq!(system_config.overhead, U256::from(0xbc));
        assert_eq!(system_config.scalar, U256::from(0xa6fe0));
        assert_eq!(system_config.gas_limit, 30000000);
    }

    #[test]
    fn test_system_config_update_with_receipts_unchanged() {
        let mut system_config = SystemConfig::default();
        let receipts = vec![];
        let l1_system_config_address = Address::ZERO;
        let ecotone_active = false;

        system_config
            .update_with_receipts(&receipts, l1_system_config_address, ecotone_active)
            .unwrap();

        assert_eq!(system_config, SystemConfig::default());
    }

    #[test]
    fn test_system_config_update_with_receipts_batcher_address() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000000");
        let mut system_config = SystemConfig::default();
        let l1_system_config_address = Address::ZERO;
        let ecotone_active = false;

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        let receipt = Receipt {
            logs: vec![update_log],
            status: Eip658Value::Eip658(true),
            cumulative_gas_used: 0,
        };

        system_config
            .update_with_receipts(&[receipt], l1_system_config_address, ecotone_active)
            .unwrap();

        assert_eq!(
            system_config.batcher_address,
            address!("000000000000000000000000000000000000bEEF"),
        );
    }

    #[test]
    fn test_system_config_update_batcher_log() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000000");

        let mut system_config = SystemConfig::default();

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        // Update the batcher address.
        system_config.process_config_update_log(&update_log, false).unwrap();

        assert_eq!(
            system_config.batcher_address,
            address!("000000000000000000000000000000000000bEEF")
        );
    }

    #[test]
    fn test_system_config_update_gas_config_log() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000001");

        let mut system_config = SystemConfig::default();

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000babe000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        // Update the batcher address.
        system_config.process_config_update_log(&update_log, false).unwrap();

        assert_eq!(system_config.overhead, U256::from(0xbabe));
        assert_eq!(system_config.scalar, U256::from(0xbeef));
    }

    #[test]
    fn test_system_config_update_gas_config_log_ecotone() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000001");

        let mut system_config = SystemConfig::default();

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000babe000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        // Update the gas limit.
        system_config.process_config_update_log(&update_log, true).unwrap();

        assert_eq!(system_config.overhead, U256::from(0));
        assert_eq!(system_config.scalar, U256::from(0xbeef));
    }

    #[test]
    fn test_system_config_update_gas_limit_log() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000002");

        let mut system_config = SystemConfig::default();

        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000beef").into()
            )
        };

        // Update the gas limit.
        system_config.process_config_update_log(&update_log, false).unwrap();

        assert_eq!(system_config.gas_limit, 0xbeef_u64);
    }

    #[test]
    fn test_system_config_update_eip1559_params_log() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000004");

        let mut system_config = SystemConfig::default();
        let update_log = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000babe0000beef").into()
            )
        };

        // Update the EIP-1559 parameters.
        system_config.process_config_update_log(&update_log, false).unwrap();

        assert_eq!(system_config.eip1559_denominator, Some(0xbabe_u32));
        assert_eq!(system_config.eip1559_elasticity, Some(0xbeef_u32));
    }

    #[test]
    fn test_system_config_update_operator_fee_log() {
        const UPDATE_TYPE: B256 =
            b256!("0000000000000000000000000000000000000000000000000000000000000005");

        let mut system_config = SystemConfig::default();
        let update_log  = Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    CONFIG_UPDATE_TOPIC,
                    CONFIG_UPDATE_EVENT_VERSION_0,
                    UPDATE_TYPE,
                ],
                hex!("0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000babe000000000000beef").into()
            )
        };

        // Update the operator fee.
        system_config.process_config_update_log(&update_log, false).unwrap();

        assert_eq!(system_config.operator_fee_scalar, Some(0xbabe_u32));
        assert_eq!(system_config.operator_fee_constant, Some(0xbeef_u64));
    }
}
