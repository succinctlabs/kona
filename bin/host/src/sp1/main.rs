//! Main entrypoint for SP1 the host binary.

use kona_host::{DiskKeyValueStore, MemoryKeyValueStore};
use op_succinct_client_utils::InMemoryOracle;
use op_succinct_host_utils::get_proof_stdin;
use sp1_sdk::{include_elf, ProverClient};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let elf = include_elf!("op-succinct");
    let disk_kv_store = DiskKeyValueStore::new("./data".into());
    let mem_kv_store = MemoryKeyValueStore::try_from(disk_kv_store)?;
    let oracle = InMemoryOracle::from(mem_kv_store);
    let stdin = get_proof_stdin(oracle)?;
    let prover = ProverClient::builder().cpu().build();

    let (_, execution_report) = prover.execute(elf, &stdin).run()?;

    println!("{execution_report}");

    Ok(())
}
