//! A program to verify a Optimism L2 block STF in the zkVM.

#![no_std]
#![cfg_attr(target_os = "zkvm", no_main)]

mod boot;
mod hint;
mod l1;
mod l2;
mod oracle;

use core::num::Wrapping;

// use boot::BootInfo;
use hint::HintType;
use kona_client::CachingOracle;
// use l1::DerivationDriver;
// use l2::OracleL2ChainProvider;
// use oracle::{Oracle, HINT_WRITER};
use oracle::InMemoryOracle;

use kona_executor::StatelessL2BlockExecutor;
use kona_primitives::L2AttributesWithParent;

use alloc::sync::Arc;
use alloy_consensus::Header;
use cfg_if::cfg_if;
use kona_client::{
    l1::{DerivationDriver, OracleBlobProvider, OracleL1ChainProvider},
    l2::{OracleL2ChainProvider, TrieDBHintWriter},
    BootInfo,
};
use tracing::trace;

extern crate alloc;

cfg_if! {
    // If the target OS is zkVM, set everything up to use no hints, read input data
    // from SP1, and compile to a program that can be run in zkVM.
    if #[cfg(target_os = "zkvm")] {
        sp1_zkvm::entrypoint!(main);
        use alloc::vec::Vec;
        use kona_mpt::NoopTrieDBHinter;
        use boot::BootInfoWithoutRollupConfig;

    // Otherwise, import the hinter and oracle LRU size to prepare for online mode.
    } else {
        const ORACLE_LRU_SIZE: usize = 1024;
    }
}

fn main() {
    kona_common::block_on(async move {
        ////////////////////////////////////////////////////////////////
        //                          PROLOGUE                          //
        ////////////////////////////////////////////////////////////////

        cfg_if! {
            // If we are compiling for the zkVM, read inputs from SP1 to generate boot info
            // and in memory oracle. We can use the no-op hinter, as we shouldn't need hints.
            if #[cfg(target_os = "zkvm")] {
                let boot_info = sp1_zkvm::io::read::<BootInfoWithoutRollupConfig>();
                sp1_zkvm::io::commit(&boot_info);
                let boot_info: Arc<BootInfo> = Arc::new(boot_info.into());

                let kv_store_bytes: Vec<u8> = sp1_zkvm::io::read_vec();
                let oracle = Arc::new(InMemoryOracle::from_raw_bytes(kv_store_bytes));
                let hinter = NoopTrieDBHinter;

            // If we are compiling for online mode, create a caching oracle that speaks to the
            // fetcher via hints, and gather boot info from this oracle.
            } else {
                let oracle = Arc::new(CachingOracle::new(ORACLE_LRU_SIZE));
                let boot_info = Arc::new(BootInfo::load(oracle.as_ref()).await.unwrap());
                let hinter = TrieDBHintWriter;
            }
        }

        let l1_provider = OracleL1ChainProvider::new(boot_info.clone(), oracle.clone());
        let l2_provider = OracleL2ChainProvider::new(boot_info.clone(), oracle.clone());
        let beacon = OracleBlobProvider::new(oracle.clone());

        cfg_if! {
        if #[cfg(target_os = "zkvm")] {
                let l1_provider = WrappingOracleL1ChainProvider::new(l1_provider);
                let l2_provider = WrappingOracleL2ChainProvider::new(l2_provider);
                let beacon = WrappingOracleBlobProvider::new(beacon);
            }
        }
        ////////////////////////////////////////////////////////////////
        //                   DERIVATION & EXECUTION                   //
        ////////////////////////////////////////////////////////////////

        let mut driver = DerivationDriver::new(
            boot_info.as_ref(),
            oracle.as_ref(),
            beacon,
            l1_provider,
            l2_provider.clone(),
        )
        .await
        .unwrap();

        let L2AttributesWithParent { attributes, .. } =
            driver.produce_disputed_payload().await.unwrap();

        let mut executor = StatelessL2BlockExecutor::new(
            &boot_info.rollup_config,
            driver.take_l2_safe_head_header(),
            l2_provider,
            hinter,
        );
        let Header { number, .. } = *executor.execute_payload(attributes).unwrap();
        let output_root = executor.compute_output_root().unwrap();

        ////////////////////////////////////////////////////////////////
        //                          EPILOGUE                          //
        ////////////////////////////////////////////////////////////////

        assert_eq!(number, boot_info.l2_claim_block);
        assert_eq!(output_root, boot_info.l2_claim);
    });
}
