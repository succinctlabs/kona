#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/op-rs/kona/main/assets/square.png",
    html_favicon_url = "https://raw.githubusercontent.com/op-rs/kona/main/assets/favicon.ico"
)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

mod clap;
pub use clap::cli_styles;

pub mod cli_parsers;

mod tracing;
pub use tracing::init_tracing_subscriber;

mod prometheus;
pub use prometheus::init_prometheus_server;

pub mod sigsegv_handler;
