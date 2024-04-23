
use kona_derive::prelude::*;
use kona_derive::types::{L2BlockInfo};

/// A simple store to hold the current block info.
#[derive(Debug)]
pub struct LocalReset {
    /// The current block info.
    block_info: Option<BlockInfo>,
}

fn main() {
    // Initialize tracing.
    tracing_subscriber::Registry::default().init();

    // Start the cursor at the first block.
    let cursor = L2BlockInfo::default();
    let reset

    // Create a new pipeline.

    let pipeline = DerivationPipeline::new(stack, reset, cursor);
}
