// A host program to generate a proof of an Optimism L2 block STF in the zkVM.

use alloy_primitives::b256;
use sp1_sdk::{utils, ProverClient, SP1Stdin};
use zkvm_client::BootInfoWithoutRollupConfig;
use serde::{Serialize, Deserialize};
use kona_preimage::PreimageKey;
use hashbrown::HashMap;
use std::{
    fs,
    io::Read
};
use hex;

const ELF: &[u8] = include_bytes!("../../../elf/riscv32im-succinct-zkvm-elf");


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InMemoryOracle {
    cache: HashMap<PreimageKey, Vec<u8>>,
}

fn main() {
    utils::setup_logger();
    let mut stdin = SP1Stdin::new();

    // Commit to public values for all data that will be verified on chain.

    let l1_head = b256!("be3e8018cedf5495244956a2fc9e21c24ef8dbf9a2b6f5f258ea52c58186defc");
    let l2_output_root = b256!("a8bf8e6642a22da7f241ad21c15ad12656c6a2cd0a8aa9765d3436ddf20ee9cb");
    let l2_claim = b256!("69bb5bf356632be020f60117092c37d320ee7d5673d0b1ff6426271adb537ec1");
    let l2_claim_block = 121572792;
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
    let kv_store = load_kv_store("../../data");
    let kv_store_bytes = bincode::serialize(&kv_store).unwrap();
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

fn load_kv_store(data_dir: &str) -> InMemoryOracle {
    let mut cache = HashMap::new();

    // Iterate over the files in the 'data' directory
    for entry in fs::read_dir(data_dir).expect("Failed to read data directory") {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() {
                // Extract the file name
                let file_name = path.file_stem().unwrap().to_str().unwrap();

                // Convert the file name to PreimageKey
                if let Ok(key_bytes) = hex::decode(file_name) {
                    if let Ok(key_array) = TryInto::<[u8;32]>::try_into(key_bytes.as_slice()) {
                        if let Ok(key) = PreimageKey::try_from(key_array) {
                            // Read the file contents
                            let mut file = fs::File::open(path).expect("Failed to open file");
                            let mut contents = Vec::new();
                            file.read_to_end(&mut contents).expect("Failed to read file");

                            // Insert the key-value pair into the cache
                            cache.insert(key, contents);
                        }
                    }
                }
            }
        }
    }

    InMemoryOracle { cache }
}
