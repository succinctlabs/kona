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
    // This data is taken from the test [test_l2_block_executor_small_block] in the [kona-client]
    // crate.
    // TODO: This calls relative to being in this directory, which fails if we call `cargo --bin zkvm-host` from root.
    let testdata_folder = "block_120794432_exec";
    let file_name = format!("../../crates/executor/testdata/{}/output.json", testdata_folder);
    let fetcher = ZkvmTrieDBFetcher::from_file(&file_name);

    // Make a mock rollup config, with Ecotone activated at timestamp = 0.
    let rollup_config = OP_MAINNET_CONFIG;

    // Decode the headers.
    let raw_header = hex!("f90244a0ff7c6abc94edcaddd02c12ec7d85ffbb3ba293f3b76897e4adece57e692bcc39a01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944200000000000000000000000000000000000011a0a0b24abb13d6149947247a8817517971bb8d213de1e23225e2b20d36a5b6427ca0c31e4a2ada52ac698643357ca89ef2740d384076ef0e17b653bcb6ea7dd8902ea09f4fcf34e78afc216240e3faa72c822f8eea4757932eb9e0fd42839d192bb903b901000440000210068007000000940000000220000006000820048404800002000004040100001b2000008800001040000018280000400001200004000101086000000802800080004008010001080000200100a00000204840000118042080000400804001000a0400080200111000000800050000020200064000000012000800048000000000101800200002000000080008001581402002200210341089000080c2d004106000000018000000804285800800000020000180008000020000000000020103410400000000200400008000280400000100020000002002000021000811000920808000010000000200210400000020008000400000000000211008808407332d3f8401c9c3808327c44d84665a343780a0edba75784acf3165bffd96df8b78ffdb3781db91f886f22b4bee0a6f722df93988000000000000000083202ef8a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4218080a0917693152c4a041efbc196e9d169087093336da96a8bb3af1e55fce447a7b8a9");
    let header = Header::decode(&mut &raw_header[..]).unwrap();
    let raw_expected_header = hex!("f90243a09506905902f5c3613c5441a8697c09e7aafdb64082924d8bd2857f9e34a47a9aa01dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347944200000000000000000000000000000000000011a0a1e9207c3c68cd4854074f08226a3643debed27e45bf1b22ab528f8de16245eda0121e8765953af84974b845fd9b01f5ff9b0f7d2886a2464535e8e9976a1c8daba092c6a5e34d7296d63d1698258c40539a20080c668fc9d63332363cfbdfa37976b9010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000808407332d408401c9c38082ab4b84665a343980a0edba75784acf3165bffd96df8b78ffdb3781db91f886f22b4bee0a6f722df93988000000000000000083201f31a056e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b4218080a0917693152c4a041efbc196e9d169087093336da96a8bb3af1e55fce447a7b8a9");
    let expected_header = Header::decode(&mut &raw_expected_header[..]).unwrap();


    let hinter = ZkvmTrieDBHinter {};

    // Initialize the block executor on block #120794431's post-state.
    let mut l2_block_executor = StatelessL2BlockExecutor::new(
        &rollup_config,
        header.seal_slow(),
        fetcher.clone(),
        hinter,
    );

    let raw_tx = hex!("7ef8f8a003b511b9b71520cd62cad3b5fd5b1b8eaebd658447723c31c7f1eba87cfe98c894deaddeaddeaddeaddeaddeaddeaddeaddead00019442000000000000000000000000000000000000158080830f424080b8a4440a5e2000000558000c5fc5000000000000000300000000665a33a70000000001310e960000000000000000000000000000000000000000000000000000000214d2697300000000000000000000000000000000000000000000000000000000000000015346d208a396843018a2e666c8e7832067358433fb87ca421273c6a4e69f78d50000000000000000000000006887246668a3b87f54deb3b94ba47a6f63f32985");
    let payload_attrs = L2PayloadAttributes {
        fee_recipient: address!("4200000000000000000000000000000000000011"),
        gas_limit: Some(0x1c9c380),
        timestamp: 0x665a3439,
        prev_randao: b256!("edba75784acf3165bffd96df8b78ffdb3781db91f886f22b4bee0a6f722df939"),
        withdrawals: Default::default(),
        parent_beacon_block_root: Some(b256!(
            "917693152c4a041efbc196e9d169087093336da96a8bb3af1e55fce447a7b8a9"
        )),
        transactions: vec![raw_tx.into()],
        no_tx_pool: false,
    };
    let produced_header = l2_block_executor.execute_payload(payload_attrs.clone()).unwrap().clone();

    assert_eq!(produced_header, expected_header);

    let fetcher_clone = fetcher.clone();
    let payload_attrs_clone = payload_attrs.clone();

    let mut stdin = SP1Stdin::new();
    stdin.write_slice(&raw_header);
    stdin.write_slice(&raw_expected_header);
    stdin.write(&fetcher_clone);
    stdin.write(&payload_attrs_clone);

    // For some reason, the RollupConfig does not like being serialized by bincode.
    // let encoded: Vec<u8> = bincode::serialize(&rollup_config).unwrap();
    // let decoded: RollupConfig = bincode::deserialize(&encoded[..]).unwrap();

    utils::setup_logger();

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
