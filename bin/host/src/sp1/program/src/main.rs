#![no_main]
sp1_zkvm::entrypoint!(main);

extern crate alloc;

use alloc::sync::Arc;

use op_succinct_client_utils::{
    boot::BootInfoStruct, client::run_opsuccinct_client, precompiles::zkvm_handle_register,
};

use alloc::vec::Vec;
use op_succinct_client_utils::InMemoryOracle;

pub fn main() {
    kona_proof::block_on(async move {
        ////////////////////////////////////////////////////////////////
        //                          PROLOGUE                          //
        ////////////////////////////////////////////////////////////////
        let in_memory_oracle_bytes: Vec<u8> = sp1_zkvm::io::read_vec();
        let oracle = Arc::new(InMemoryOracle::from_raw_bytes(in_memory_oracle_bytes));

        println!("cycle-tracker-report-start: oracle-verify");
        oracle.verify().expect("key value verification failed");
        println!("cycle-tracker-report-end: oracle-verify");

        let boot_info = run_opsuccinct_client(oracle, Some(zkvm_handle_register))
            .await
            .expect("failed to run client");

        sp1_zkvm::io::commit(&BootInfoStruct::from(boot_info));
    });
}
