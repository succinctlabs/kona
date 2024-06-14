use crate::{BasicKernelInterface, FileDescriptor};
use anyhow::Result;

/// Concrete implementation of the [`KernelIO`] trait for the `SP1` target architecture.
#[derive(Debug)]
pub struct ZkvmIO;

impl BasicKernelInterface for ZkvmIO {
    fn write(fd: FileDescriptor, buf: &[u8]) -> Result<usize> {
        // TODO: Implement this function.
        Ok(0)
    }

    fn read(fd: FileDescriptor, buf: &mut [u8]) -> Result<usize> {
        // TODO: Implement this function.
        Ok(0)
    }

    fn exit(code: usize) -> ! {
        // TODO: Implement this function.
        panic!()
    }
}
