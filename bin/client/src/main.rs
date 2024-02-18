#![doc = include_str!("../README.md")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(target_arch = "mips", feature(asm_experimental_arch))]
#![no_std]
#![no_main]

#[cfg(any(target_arch = "mips", target_arch = "riscv64"))]
extern crate alloc;

use boot::BootInfo;
use kona_common::io;
use kona_preimage::OracleReader;

mod boot;
mod constants;
mod types;

/// The entry point for the client program.
fn boot() {
    kona_common::alloc_heap!(constants::HEAP_SIZE);

    let oracle = OracleReader::new(constants::CLIENT_PREIMAGE_PIPE);
    let _ = BootInfo::try_boot(&oracle).expect("Failed to boot client program");

    // Exit the program with a success code.
    io::exit(0);
}

/// The entry point for the client program when ran on a bare-metal target. Aliases `boot`.
#[cfg(any(target_arch = "mips", target_arch = "riscv64"))]
#[no_mangle]
pub extern "C" fn _start() {
    boot();
}

/// The entry point for the client program when ran natively. Aliases `boot`.
#[cfg(not(any(target_arch = "mips", target_arch = "riscv64")))]
#[no_mangle]
pub fn main() {
    boot();
}

#[cfg(any(target_arch = "mips", target_arch = "riscv64"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    let msg = alloc::format!("Panic: {}", info);
    io::print_err(msg.as_ref());
    io::exit(2)
}
