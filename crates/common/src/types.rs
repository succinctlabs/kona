//! This module contains the local types for the `kona-common` crate.

use std::{fs::File, os::fd::AsRawFd};

/// File descriptors available to the `client` within the FPVM kernel.
#[cfg_attr(not(feature = "std"), derive(Clone))]
#[derive(Debug)]
pub enum FileDescriptor {
    /// Read-only standard input stream.
    StdIn,
    /// Write-only standaard output stream.
    StdOut,
    /// Write-only standard error stream.
    StdErr,
    /// Read-only. Used to read the status of pre-image hinting.
    HintRead,
    /// Write-only. Used to provide pre-image hints
    HintWrite,
    /// Read-only. Used to read pre-images.
    PreimageRead,
    /// Write-only. Used to request pre-images.
    PreimageWrite,
    #[cfg(feature = "std")]
    /// Other file descriptor.
    Wildcard(File),
}

#[cfg(feature = "std")]
impl Clone for FileDescriptor {
    fn clone(&self) -> Self {
        match self {
            FileDescriptor::Wildcard(file) => FileDescriptor::Wildcard(file.try_clone().unwrap()),
            _ => self.clone(),
        }
    }
}

impl From<FileDescriptor> for usize {
    fn from(fd: FileDescriptor) -> Self {
        match fd {
            FileDescriptor::StdIn => 0,
            FileDescriptor::StdOut => 1,
            FileDescriptor::StdErr => 2,
            FileDescriptor::HintRead => 3,
            FileDescriptor::HintWrite => 4,
            FileDescriptor::PreimageRead => 5,
            FileDescriptor::PreimageWrite => 6,
            #[cfg(feature = "std")]
            FileDescriptor::Wildcard(value) => value.as_raw_fd() as usize,
        }
    }
}

impl From<FileDescriptor> for i32 {
    fn from(fd: FileDescriptor) -> Self {
        usize::from(fd) as Self
    }
}
