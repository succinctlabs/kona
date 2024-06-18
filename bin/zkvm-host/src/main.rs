// A host program to generate a proof of an Optimism L2 block STF in the zkVM.

use alloy_consensus::{Header, Sealable};
use alloy_primitives::{address, b256, hex};
use alloy_rlp::Decodable;
use kona_executor::StatelessL2BlockExecutor;
use kona_derive::types::{L2PayloadAttributes, RollupConfig, OP_MAINNET_CONFIG};
use kona_zkvm::{ZkvmTrieDBFetcher, ZkvmTrieDBHinter};

use sp1_sdk::{utils, ProverClient, SP1Stdin};

const ELF: &[u8] = include_bytes!("../../../elf/riscv32im-succinct-zkvm-elf");

fn main() {
    utils::setup_logger();

    let mut stdin = SP1Stdin::new();

    // Commit to public values for all data that will be verified on chain.

    let l1_head = b256::from_hex("0x1234").unwrap();
    stdin.write(&l1_head);

    let l2_output_root = b256::from_hex("0x5678").unwrap();
    stdin.write(&l2_output_root);

    let l2_claim = b256::from_hex("0x9abc").unwrap();
    stdin.write(&l2_claim);

    let l2_claim_block = 0x1234;
    stdin.write(&l2_claim_block);

    let chain_id = 10;
    stdin.write(&chain_id);

    // Read KV store into raw bytes and pass to stdin.

    let dir_path = "../../../data/"
    let kv_store_bytes = std::fs::read(format!("{}/kv.bin", dir_path)).unwrap();
    stdin.write_slice(&kv_store_bytes);

    // First instantiate a mock prover client to just execute the program and get the estimation of
    // cycle count.
    let client = ProverClient::mock();

    let (mut public_values, report) = client.execute(ELF, stdin).unwrap();
    println!("Report: {}", report);

    // Then generate the real proof.
    // let (pk, vk) = client.setup(ELF);
    // let mut proof = client.prove(&pk, stdin).unwrap();

    println!("generated proof");
}
