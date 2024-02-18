//! Contains the [HintWriter] type, which is a high-level interface to the hint writer pipe.

use crate::{traits::HintWriterClient, PipeHandle};
use alloc::vec;
use anyhow::Result;

/// A [HintWriter] is a high-level interface to the hint pipe. It provides a way to write hints to the host.
#[derive(Debug, Clone, Copy)]
pub struct HintWriter {
    pipe_handle: PipeHandle,
}

impl HintWriter {
    /// Create a new [HintWriter] from a [PipeHandle].
    pub fn new(pipe_handle: PipeHandle) -> Self {
        Self { pipe_handle }
    }
}

impl HintWriterClient for HintWriter {
    /// Write a hint to the host. This will overwrite any existing hint in the pipe, and block until all data has been
    /// written.
    fn write(&self, hint: &str) -> Result<()> {
        // Form the hint into a byte buffer. The format is a 4-byte big-endian length prefix followed by the hint
        // string.
        let mut hint_bytes = vec![0u8; hint.len() + 4];
        hint_bytes[0..4].copy_from_slice(u32::to_be_bytes(hint.len() as u32).as_ref());
        hint_bytes[4..].copy_from_slice(hint.as_bytes());

        // Write the hint to the host.
        self.pipe_handle.write(&hint_bytes)?;

        // Read the hint acknowledgement from the host.
        let mut hint_ack = [0u8; 1];
        self.pipe_handle.read_exact(&mut hint_ack)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use kona_common::FileDescriptor;
    use std::{fs::File, os::fd::AsRawFd};
    use tempfile::tempfile;

    /// Test struct containing the [HintWriter] and a [PipeHandle] for the host, plus the open [File]s. The [File]s
    /// are stored in this struct so that they are not dropped until the end of the test.
    ///
    /// TODO: Swap host pipe handle to hint router once it exists.
    #[derive(Debug)]
    struct ClientAndHost {
        hint_writer: HintWriter,
        host_handle: PipeHandle,
        _read_file: File,
        _write_file: File,
    }

    /// Helper for creating a new [HintWriter] and [PipeHandle] for testing. The file channel is over two temporary
    /// files.
    ///
    /// TODO: Swap host pipe handle to hint router once it exists.
    fn client_and_host() -> ClientAndHost {
        let (read_file, write_file) = (tempfile().unwrap(), tempfile().unwrap());
        let (read_fd, write_fd) = (
            FileDescriptor::Wildcard(read_file.as_raw_fd().try_into().unwrap()),
            FileDescriptor::Wildcard(write_file.as_raw_fd().try_into().unwrap()),
        );
        let client_handle = PipeHandle::new(read_fd, write_fd);
        let host_handle = PipeHandle::new(write_fd, read_fd);

        let hint_writer = HintWriter::new(client_handle);

        ClientAndHost {
            hint_writer,
            host_handle,
            _read_file: read_file,
            _write_file: write_file,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_hint_writer() {
        const MOCK_DATA: &str = "dummy-hint facade";
        let sys = client_and_host();
        let (hint_writer, host_handle) = (sys.hint_writer, sys.host_handle);

        let client = tokio::task::spawn(async move {
            hint_writer.write(MOCK_DATA).unwrap();
        });
        let host = tokio::task::spawn(async move {
            let mut hint_bytes = vec![0u8; MOCK_DATA.len() + 4];
            host_handle.read_exact(hint_bytes.as_mut_slice()).unwrap();

            let len = u32::from_be_bytes(hint_bytes[..4].try_into().unwrap());
            assert_eq!(len, MOCK_DATA.len() as u32);
            assert_eq!(&hint_bytes[4..], MOCK_DATA.as_bytes());

            let ack = [1u8; 1];
            host_handle.write(&ack).unwrap();
        });

        let (r, w) = tokio::join!(client, host);
        r.unwrap();
        w.unwrap();
    }
}
