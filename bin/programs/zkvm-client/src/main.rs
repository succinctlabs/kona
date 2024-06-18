//! A program to verify a Optimism L2 block STF in the zkVM.

#![no_std]
#![cfg_attr(target_os = "zkvm", no_main)]

mod l1;
mod l2;
mod boot;
mod oracle;
mod hint;

use l1::{DerivationDriver, OracleBlobProvider, OracleL1ChainProvider};
use l2::OracleL2ChainProvider;
use boot::BootInfo;
use oracle::{Oracle, InMemoryOracle, CachingOracle, HINT_WRITER};
use hint::HintType;

use kona_primitives::L2AttributesWithParent;
use kona_executor::StatelessL2BlockExecutor;

use cfg_if::cfg_if;
use alloc::sync::Arc;
use alloy_consensus::Header;

extern crate alloc;

/// The size of the LRU cache in the oracle.
const ORACLE_LRU_SIZE: usize = 1024;

cfg_if! {
    if #[cfg(target_os = "zkvm")] {
        sp1_zkvm::entrypoint!(main);
        use alloc::vec::Vec;
        use kona_mpt::NoopTrieDBHinter;
        use boot::BootInfoWithoutRollupConfig;
    } else {
        use l2::TrieDBHintWriter;
    }
}

fn main() {

    kona_common::block_on(async move {

        ////////////////////////////////////////////////////////////////
        //                          PROLOGUE                          //
        ////////////////////////////////////////////////////////////////

        cfg_if! {
            if #[cfg(target_os = "zkvm")] {
                #[doc = "Concrete implementation of the [BasicKernelInterface] trait for the `zkvm` target architecture."]

                let boot_info = sp1_zkvm::io::read::<BootInfoWithoutRollupConfig>();
                sp1_zkvm::io::commit(&boot_info);
                let boot_info: Arc<BootInfo> = Arc::new(boot_info.into());

                let kv_store_bytes: Vec<u8> = sp1_zkvm::io::read_vec();
                let oracle = Arc::new(Oracle::new_in_memory(kv_store_bytes));
                let hinter = NoopTrieDBHinter;
            } else {
                let oracle = Arc::new(Oracle::new_caching(ORACLE_LRU_SIZE));
                let boot_info = Arc::new(BootInfo::load(oracle.as_ref()).await.unwrap());
                let hinter = TrieDBHintWriter;
            }
        }

        let l1_provider = OracleL1ChainProvider::new(boot_info.clone(), oracle.clone());
        let l2_provider = OracleL2ChainProvider::new(boot_info.clone(), oracle.clone());
        let beacon = OracleBlobProvider::new(oracle.clone());

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

        let L2AttributesWithParent { attributes, .. } = driver.produce_disputed_payload().await.unwrap();

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
