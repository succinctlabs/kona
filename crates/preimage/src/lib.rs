#![doc = include_str!("../README.md")]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub, rustdoc::all)]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![no_std]

extern crate alloc;

mod key;
pub use key::{PreimageKey, PreimageKeyType};

#[cfg(not(feature = "no-io"))]
mod oracle;
#[cfg(not(feature = "no-io"))]
pub use oracle::{OracleReader, OracleServer};

#[cfg(not(feature = "no-io"))]
mod hint;
#[cfg(not(feature = "no-io"))]
pub use hint::{HintReader, HintWriter};

#[cfg(not(feature = "no-io"))]
mod pipe;
#[cfg(not(feature = "no-io"))]
pub use pipe::PipeHandle;

mod traits;
pub use traits::{
    CommsClient, HintReaderServer, HintRouter, HintWriterClient, PreimageFetcher,
    PreimageOracleClient, PreimageOracleServer,
};
