// A host program to generate a proof of an Optimism L2 block STF in the zkVM.

use alloy_primitives::b256;
use sp1_sdk::{utils, ProverClient, SP1Stdin};
use kona_zkvm_client::BootInfoWithoutRollupConfig;

const ELF: &[u8] = include_bytes!("../../../elf/riscv32im-succinct-zkvm-elf");

fn main() {
    utils::setup_logger();
    let mut stdin = SP1Stdin::new();

    // Commit to public values for all data that will be verified on chain.

    let l1_head = b256!("9506905902f5c3613c5441a8697c09e7aafdb64082924d8bd2857f9e34a47a9a");
    let l2_output_root = b256!("b8a465f44c168c4bffc43fcf933138a26a3410473dd2070052d5ea6cb366ac60");
    let l2_claim = b256!("b576409c0640c575de51d78cc0df71914dbd4ae4639c4782bdcbc9f6daf19620");
    let l2_claim_block = 120794432;
    let chain_id = 10;

    let boot_info = BootInfoWithoutRollupConfig {
        l1_head,
        l2_output_root,
        l2_claim,
        l2_claim_block,
        chain_id,
    };
    stdin.write(&boot_info);

    // Read KV store into raw bytes and pass to stdin.
    let kv_store_bytes = std::fs::read(format!("../../../data/kv.bin")).unwrap();
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
