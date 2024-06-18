//! A program to verify a Optimism L2 block STF in the zkVM.

#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_consensus::{Header, Sealable};
use alloy_rlp::Decodable;
use kona_executor::StatelessL2BlockExecutor;
use kona_derive::types::{L2PayloadAttributes, OP_MAINNET_CONFIG};
use kona_zkvm::{ZkvmTrieDBFetcher, ZkvmTrieDBHinter};

pub fn main() {
    let rollup_config = OP_MAINNET_CONFIG;

    let raw_header = sp1_zkvm::io::read_vec();
    let header = Header::decode(&mut &raw_header[..]).unwrap();

    let raw_expected_header = sp1_zkvm::io::read_vec();
    let expected_header = Header::decode(&mut &raw_expected_header[..]).unwrap();

    let fetcher: ZkvmTrieDBFetcher = sp1_zkvm::io::read();
    println!("Verifying fetcher...");
    fetcher.verify();
    println!("Done verifying fetcher.");

    let hinter = ZkvmTrieDBHinter {};
    let payload_attrs: L2PayloadAttributes = sp1_zkvm::io::read();

    let mut l2_block_executor =
        StatelessL2BlockExecutor::new(&rollup_config, header.seal_slow(), fetcher, hinter);
    println!("Initialized block executor.");
    let produced_header = l2_block_executor.execute_payload(payload_attrs).unwrap().clone();
    println!("Executed payload.");
    assert_eq!(produced_header, expected_header);
    println!("Assertion passed.");

    // TODO: assert that the block executor's state is correct.
    // assert_eq!(
    //     l2_block_executor.state.database.parent_block_header().seal(),
    //     expected_header.hash_slow()
    // );
}
