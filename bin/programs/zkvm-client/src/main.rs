//! A program to verify a Optimism L2 block STF in the zkVM.

#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

#![no_main]
sp1_zkvm::entrypoint!(main);

mod l1;
mod l2;
mod boot;
mod comms;

use alloc::{sync::Arc, vec::Vec};
use alloy_consensus::Header;
use kona_mpt::NoopTrieDBHinter;
use l1::{DerivationDriver, OracleBlobProvider, OracleL1ChainProvider};
use l2::{OracleL2ChainProvider, TrieDBHintWriter};
use boot::{BootInfo, BootInfoWithoutRollupConfig};
use comms::InMemoryOracle;
use kona_primitives::L2AttributesWithParent;
use kona_executor::StatelessL2BlockExecutor;
use cfg_if::cfg_if;

extern crate alloc;

/// The size of the LRU cache in the oracle.
const ORACLE_LRU_SIZE: usize = 1024;

// TODO: How does this work when we do ZKVM run? Is it compatible with zkvm::entrypoint, or need some cfg stuff?
#[client_entry(0x77359400)]
fn main() {

    kona_common::block_on(async move {

        ////////////////////////////////////////////////////////////////
        //                          PROLOGUE                          //
        ////////////////////////////////////////////////////////////////

        cfg_if! {
            if #[cfg(target_os = "zkvm")] {
                #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `zkvm` target architecture."]

                let boot = sp1_zkvm::io::read<BootInfoWithoutRollupConfig>();
                sp1_zkvm::io::commit(&boot);
                let boot: Arc<BootInfo> = Arc::new(boot.into());

                let kv_store_bytes: Vec<u8> = sp1_zkvm::io::read_vec();
                let oracle = Arc::new(InMemoryOracle::from_raw_bytes(kv_store_bytes));
                let hinter = NoopTrieDBHinter;
            } else {
                let oracle = Arc::new(CachingOracle::new(ORACLE_LRU_SIZE));
                let boot = Arc::new(BootInfo::load(oracle.as_ref()).await?);
                let hinter = TrieDBHintWriter;
            }
        }

        let l1_provider = OracleL1ChainProvider::new(boot.clone(), oracle.clone());
        let l2_provider = OracleL2ChainProvider::new(boot.clone(), oracle.clone());
        let beacon = OracleBlobProvider::new(oracle.clone());

        ////////////////////////////////////////////////////////////////
        //                   DERIVATION & EXECUTION                   //
        ////////////////////////////////////////////////////////////////

        let mut driver = DerivationDriver::new(
            boot.as_ref(),
            oracle.as_ref(),
            beacon,
            l1_provider,
            l2_provider.clone(),
        )
        .await?;
        let L2AttributesWithParent { attributes, .. } = driver.produce_disputed_payload().await?;

        let mut executor = StatelessL2BlockExecutor::new(
            &boot.rollup_config,
            driver.take_l2_safe_head_header(),
            l2_provider,
            hinter,
        );
        let Header { number, .. } = *executor.execute_payload(attributes).unwrap();
        let output_root = executor.compute_output_root().unwrap();

        ////////////////////////////////////////////////////////////////
        //                          EPILOGUE                          //
        ////////////////////////////////////////////////////////////////

        assert_eq!(number, l2_claim_block);
        assert_eq!(output_root, l2_claim);
    }
}
