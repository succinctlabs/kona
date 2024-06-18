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
mod mem_oracle;

use alloc::{sync::Arc, vec::Vec};
use alloy_consensus::Header;
use kona_mpt::NoopTrieDBHinter;
use l1::{DerivationDriver, OracleBlobProvider, OracleL1ChainProvider};
use l2::{OracleL2ChainProvider, TrieDBHintWriter};
use boot::BootInfo;
use mem_oracle::InMemoryOracle;
use kona_primitives::L2AttributesWithParent;
use kona_executor::StatelessL2BlockExecutor;

extern crate alloc;

fn main() {

    ////////////////////////////////////////////////////////////////
    //                          PROLOGUE                          //
    ////////////////////////////////////////////////////////////////

    let kv_store_bytes: Vec<u8> = sp1_zkvm::io::read();
    let oracle = Arc::new(InMemoryOracle::from_raw_bytes(kv_store_bytes));

    let l1_head = sp1_zkvm::io::read();
    let l2_output_root = sp1_zkvm::io::read();
    let l2_claim = sp1_zkvm::io::read();
    let l2_claim_block = sp1_zkvm::io::read();
    let chain_id = sp1_zkvm::io::read();
    let boot = Arc::new(BootInfo::new(
        l1_head,
        l2_output_root,
        l2_claim,
        l2_claim_block,
        chain_id,
    ));

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
        NoopTrieDBHinter,
    );
    let Header { number, .. } = *executor.execute_payload(attributes)?;
    let output_root = executor.compute_output_root()?;

    ////////////////////////////////////////////////////////////////
    //                          EPILOGUE                          //
    ////////////////////////////////////////////////////////////////

    assert_eq!(number, l2_claim_block);
    assert_eq!(output_root, l2_claim);
}
