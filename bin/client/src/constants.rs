//! This module contains constant values used throughout the client program.

use kona_common::FileDescriptor;
use kona_preimage::PipeHandle;

/// The size of the heap to allocate, in bytes (1GB).
#[cfg(any(target_arch = "mips", target_arch = "riscv64"))]
pub(crate) const HEAP_SIZE: usize = 0x3B9ACA00;

/// The pipe handle used to communicate with the preimage oracle from the client program.
pub(crate) const CLIENT_PREIMAGE_PIPE: PipeHandle =
    PipeHandle::new(FileDescriptor::PreimageRead, FileDescriptor::PreimageWrite);
