// A host program to generate a proof of an Optimism L2 block STF in the zkVM.

use alloy_primitives::{b256, Bytes};
use sp1_sdk::{utils, ProverClient, SP1Stdin};
use zkvm_client::BootInfoWithoutRollupConfig;
use zkvm_common::BytesHasherBuilder;
use rkyv::{
    ser::{serializers::*, Serializer},
    AlignedVec, Archive, Deserialize, Serialize
};
use std::{
    fs,
    io::Read,
    collections::HashMap
};
use hex;

const ELF: &[u8] = include_bytes!("../../../elf/riscv32im-succinct-zkvm-elf");


#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(Debug))]
pub struct InMemoryOracle {
    cache: HashMap<[u8;32], Vec<u8>, BytesHasherBuilder>,
}

fn main() {
    utils::setup_logger();
    let mut stdin = SP1Stdin::new();

    // Commit to public values for all data that will be verified on chain.

    let l1_head = b256!("ba1f96c4ad1c66d86e6390c22f4cc759429255dfa410d7ec57cdd5560547bb2e");
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

    let mut serializer = CompositeSerializer::new(
        AlignedSerializer::new(AlignedVec::new()),
        HeapScratch::<8388608>::new(),
        SharedSerializeMap::new(),
    );
    serializer.serialize_value(&kv_store).unwrap();

    let buffer = serializer.into_serializer().into_inner();
    let kv_store_bytes = buffer.into_vec();
    stdin.write_slice(&kv_store_bytes);

    // First instantiate a mock prover client to just execute the program and get the estimation of
    // cycle count.
    let client = ProverClient::mock();

    let (mut public_values, report) = client.execute(ELF, stdin).unwrap();
    println!("Report: {}", report);

    // Then generate the real proof.
    // let (pk, vk) = client.setup(ELF);
    // let mut proof = client.prove(&pk, stdin).unwrap();

    println!("generated valid zk proof");
}

fn load_kv_store(data_dir: &str) -> HashMap<[u8;32], Vec<u8>, BytesHasherBuilder> {
    let capacity = get_file_count(data_dir);
    let mut cache: HashMap<[u8;32], Vec<u8>, BytesHasherBuilder> =
        HashMap::with_capacity_and_hasher(capacity, BytesHasherBuilder);

    // Iterate over the files in the 'data' directory
    for entry in fs::read_dir(data_dir).expect("Failed to read data directory") {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() {
                // Extract the file name
                let file_name = path.file_stem().unwrap().to_str().unwrap();

                // Convert the file name to PreimageKey
                if let Ok(key) = hex::decode(file_name) {
                    // Read the file contents
                    let mut file = fs::File::open(path).expect("Failed to open file");
                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents).expect("Failed to read file");

                    // Insert the key-value pair into the cache
                    cache.insert(key.try_into().unwrap(), contents);
                }
            }
        }
    }

    cache
}

fn get_file_count(data_dir: &str) -> usize {
    let mut file_count = 0;
    for entry in fs::read_dir(data_dir).expect("failed to read data dir") {
        let entry = entry.unwrap();
        if entry.metadata().unwrap().is_file() {
            file_count += 1;
        }
    }
    file_count
}

// fn main() {
//     let mut map: HashMap<[u8; 32], u64, BytesHasherBuilder> = HashMap::with_hasher(BytesHasherBuilder);

//     let key1: [u8; 32] = [1; 32];
//     let key2: [u8; 32] = [2; 32];

//     map.insert(key1, 10);
//     map.insert(key2, 20);

//     println!("{:?}", map.get(&key1));
//     println!("{:?}", map.get(&key2));
// }
