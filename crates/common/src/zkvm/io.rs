use crate::{BasicKernelInterface, FileDescriptor};
use anyhow::Result;

/// Concrete implementation of the [`KernelIO`] trait for the `SP1` target architecture.
#[derive(Debug)]
pub struct ZkvmIO;

impl BasicKernelInterface for ZkvmIO {
    fn write(_fd: FileDescriptor, _buf: &[u8]) -> Result<usize> {
        Err(anyhow::anyhow!("write not implemented"))
    }

    fn read(_fd: FileDescriptor, _buf: &mut [u8]) -> Result<usize> {
        Err(anyhow::anyhow!("read not implemented"))
    }

    fn exit(_code: usize) -> ! {
        Err(anyhow::anyhow!("exit not implemented"))
    }
}
