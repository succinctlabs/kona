//! The Span Batch Type

#![allow(unused)]

use super::{SpanBatchError, SpanBatchTransactions};
use crate::{
    traits::L2ChainProvider,
    types::{
        BatchValidity, BlockInfo, L2BlockInfo, RawSpanBatch, RollupConfig, SingleBatch,
        SpanBatchBits, SpanBatchElement, SpanBatchPayload, SpanBatchPrefix,
    },
};
use alloc::{vec, vec::Vec};
use alloy_primitives::FixedBytes;
use op_alloy_consensus::OpTxType;
use tracing::{info, warn};

/// The span batch contains the input to build a span of L2 blocks in derived form.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SpanBatch {
    /// First 20 bytes of the first block's parent hash
    pub parent_check: FixedBytes<20>,
    /// First 20 bytes of the last block's L1 origin hash
    pub l1_origin_check: FixedBytes<20>,
    /// Genesis block timestamp
    pub genesis_timestamp: u64,
    /// Chain ID
    pub chain_id: u64,
    /// List of block input in derived form
    pub batches: Vec<SpanBatchElement>,
    /// Caching - origin bits
    pub origin_bits: SpanBatchBits,
    /// Caching - block tx counts
    pub block_tx_counts: Vec<u64>,
    /// Caching - span batch txs
    pub txs: SpanBatchTransactions,
}

impl SpanBatch {
    /// Returns the timestamp for the first batch in the span.
    pub fn timestamp(&self) -> u64 {
        self.batches[0].timestamp
    }

    /// Returns the epoch number for the first batch in the span.
    pub fn starting_epoch_num(&self) -> u64 {
        self.batches[0].epoch_num
    }

    /// Checks if the first 20 bytes of the given hash match the L1 origin check.
    pub fn check_origin_hash(&self, hash: FixedBytes<32>) -> bool {
        self.l1_origin_check == hash[..20]
    }

    /// Checks if the first 20 bytes of the given hash match the parent check.
    pub fn check_parent_hash(&self, hash: FixedBytes<32>) -> bool {
        self.parent_check == hash[..20]
    }

    /// Checks if the span batch is valid.
    pub async fn check_batch<BF: L2ChainProvider>(
        &self,
        cfg: &RollupConfig,
        l1_blocks: &[BlockInfo],
        l2_safe_head: L2BlockInfo,
        inclusion_block: &BlockInfo,
        fetcher: &mut BF,
    ) -> BatchValidity {
        if l1_blocks.is_empty() {
            warn!("missing L1 block input, cannot proceed with batch checking");
            return BatchValidity::Undecided;
        }
        if self.batches.is_empty() {
            warn!("empty span batch, cannot proceed with batch checking");
            return BatchValidity::Undecided;
        }
        let epoch = l1_blocks[0];
        let mut batch_origin = epoch;
        let starting_epoch_num = self.starting_epoch_num();
        if starting_epoch_num == batch_origin.number + 1 {
            if l1_blocks.len() < 2 {
                info!("eager batch wants to advance current epoch {}, but could not without more L1 blocks", epoch.id());
                return BatchValidity::Undecided;
            }
            batch_origin = l1_blocks[1];
        }

        // Span batches are only valid after the Delta hard fork.
        if !cfg.is_delta_active(batch_origin.timestamp) {
            warn!(
                "received SpanBatch (id {}) with L1 origin (timestamp {}) before Delta hard fork",
                batch_origin.id(),
                batch_origin.timestamp
            );
            return BatchValidity::Drop;
        }

        // Skip out of order batches.
        let next_timestamp = l2_safe_head.block_info.timestamp + cfg.block_time;
        if self.timestamp() > next_timestamp {
            warn!(
                "received out-of-order batch for future processing after next batch ({} > {})",
                self.timestamp(),
                next_timestamp
            );
            return BatchValidity::Future;
        }
        // SAFETY: The span batch is not empty so the last element exists.
        if self.batches.last().unwrap().timestamp < next_timestamp {
            warn!("span batch has no new blocks after safe head");
            return BatchValidity::Drop;
        }

        // Find the parent block of the span batch.
        // If the span batch does not overlap the current safe chain, parent block should be the L2
        // safe head.
        let mut parent_num = l2_safe_head.block_info.number;
        let parent_block = l2_safe_head;
        if self.timestamp() < next_timestamp {
            if self.timestamp() > l2_safe_head.block_info.timestamp {
                // Batch timestamp cannot be between safe head and next timestamp.
                warn!("batch has misaligned timestamp, block time is too short");
                return BatchValidity::Drop;
            }
            if (l2_safe_head.block_info.timestamp - self.timestamp()) % cfg.block_time != 0 {
                warn!("batch has misaligned timestamp, not overlapped exactly");
                return BatchValidity::Drop;
            }
            parent_num = l2_safe_head.block_info.number -
                (l2_safe_head.block_info.timestamp - self.timestamp()) / cfg.block_time -
                1;
            let parent_block = match fetcher.l2_block_info_by_number(parent_num).await {
                Ok(block) => block,
                Err(e) => {
                    warn!("failed to fetch L2 block number {parent_num}: {e}");
                    // Unable to validate the batch for now. Retry later.
                    return BatchValidity::Undecided;
                }
            };
        }
        if !self.check_parent_hash(parent_block.block_info.parent_hash) {
            warn!(
                "parent block number mismatch, expected: {parent_num}, received: {}",
                parent_block.block_info.number
            );
            return BatchValidity::Drop;
        }

        // Filter out batches that were included too late.
        if starting_epoch_num + cfg.seq_window_size < inclusion_block.number {
            warn!("batch was included too late, sequence window expired");
            return BatchValidity::Drop;
        }

        // Check the L1 origin of the batch
        if starting_epoch_num > parent_block.l1_origin.number + 1 {
            warn!(
                "batch is for future epoch too far ahead, while it has the next timestamp, so it must be invalid, current_epoch: {}",
                epoch.id()
            );
            return BatchValidity::Drop;
        }

        // Verify the l1 origin hash for each l1 block.
        // SAFETY: The span batch is not empty so the last element exists.
        let end_epoch_num = self.batches.last().unwrap().epoch_num;
        let mut origin_checked = false;
        // l1Blocks is supplied from batch queue and its length is limited to SequencerWindowSize.
        for l1_block in l1_blocks {
            if l1_block.number == end_epoch_num {
                if !self.check_origin_hash(l1_block.hash) {
                    warn!(
                        "batch is for different L1 chain, epoch hash does not match, expected: {}",
                        l1_block.hash
                    );
                    return BatchValidity::Drop;
                }
                origin_checked = true;
                break;
            }
        }
        if !origin_checked {
            info!("need more l1 blocks to check entire origins of span batch");
            return BatchValidity::Undecided;
        }

        // Check if the batch is too old.
        if starting_epoch_num < parent_block.l1_origin.number {
            warn!("dropped batch, epoch is too old, minimum: {}", parent_block.block_info.id());
            return BatchValidity::Drop;
        }

        let mut origin_index = 0;
        let mut origin_advanced = starting_epoch_num == parent_block.l1_origin.number + 1;
        for (i, batch) in self.batches.iter().enumerate() {
            if batch.timestamp <= l2_safe_head.block_info.timestamp {
                continue;
            }
            // Find the L1 origin for the batch.
            for (j, j_block) in l1_blocks.iter().enumerate().skip(origin_index) {
                if batch.epoch_num == j_block.number {
                    origin_index = j;
                    break;
                }
            }
            let l1_origin = l1_blocks[origin_index];
            if i > 0 {
                origin_advanced = false;
                if batch.epoch_num > self.batches[i - 1].epoch_num {
                    origin_advanced = true;
                }
            }
            let block_timestamp = batch.timestamp;
            if block_timestamp < l1_origin.timestamp {
                warn!(
                    "block timestamp is less than L1 origin timestamp, l2_timestamp: {}, l1_timestamp: {}, origin: {}",
                    block_timestamp,
                    l1_origin.timestamp,
                    l1_origin.id()
                );
                return BatchValidity::Drop;
            }
            // Check if we ran out of sequencer time drift
            if block_timestamp > l1_origin.timestamp + cfg.max_sequencer_drift {
                if batch.transactions.is_empty() {
                    // If the sequencer is co-operating by producing an empty batch,
                    // then allow the batch if it was the right thing to do to maintain the L2 time
                    // >= L1 time invariant. We only check batches that do not
                    // advance the epoch, to ensure epoch advancement regardless of time drift is
                    // allowed.
                    if !origin_advanced {
                        if origin_index + 1 >= l1_blocks.len() {
                            info!("without the next L1 origin we cannot determine yet if this empty batch that exceeds the time drift is still valid");
                            return BatchValidity::Undecided;
                        }
                        if block_timestamp >= l1_blocks[origin_index + 1].timestamp {
                            // check if the next L1 origin could have been adopted
                            info!("batch exceeded sequencer time drift without adopting next origin, and next L1 origin would have been valid");
                            return BatchValidity::Drop;
                        } else {
                            info!("continuing with empty batch before late L1 block to preserve L2 time invariant");
                        }
                    }
                } else {
                    // If the sequencer is ignoring the time drift rule, then drop the batch and
                    // force an empty batch instead, as the sequencer is not
                    // allowed to include anything past this point without moving to the next epoch.
                    warn!(
                        "batch exceeded sequencer time drift, sequencer must adopt new L1 origin to include transactions again, max_time: {}",
                        l1_origin.timestamp + cfg.max_sequencer_drift
                    );
                    return BatchValidity::Drop;
                }
            }

            // Check that the transactions are not empty and do not contain any deposits.
            for (tx_index, tx_bytes) in batch.transactions.iter().enumerate() {
                if tx_bytes.is_empty() {
                    warn!(
                        "transaction data must not be empty, but found empty tx, tx_index: {}",
                        tx_index
                    );
                    return BatchValidity::Drop;
                }
                if tx_bytes.0[0] == OpTxType::Deposit as u8 {
                    warn!("sequencers may not embed any deposits into batch data, but found tx that has one, tx_index: {}", tx_index);
                    return BatchValidity::Drop;
                }
            }
        }

        // Check overlapped blocks
        if self.timestamp() < next_timestamp {
            for i in 0..(l2_safe_head.block_info.number - parent_num) {
                let safe_block_num = parent_num + i + 1;
                let safe_block_payload = match fetcher.payload_by_number(safe_block_num).await {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("failed to fetch payload for block number {safe_block_num}: {e}");
                        return BatchValidity::Undecided;
                    }
                };
                let safe_block_txs = &safe_block_payload.execution_payload.transactions;
                let batch_txs = &self.batches[i as usize].transactions;
                // Execution payload has deposit txs but batch does not.
                let deposit_count: usize = safe_block_txs
                    .iter()
                    .map(|tx| if tx.0[0] == OpTxType::Deposit as u8 { 1 } else { 0 })
                    .sum();
                if safe_block_txs.len() - deposit_count != batch_txs.len() {
                    warn!(
                        "overlapped block's tx count does not match, safe_block_txs: {}, batch_txs: {}",
                        safe_block_txs.len(),
                        batch_txs.len()
                    );
                    return BatchValidity::Drop;
                }
                for j in 0..batch_txs.len() {
                    if safe_block_txs[j + deposit_count] != batch_txs[j].0 {
                        warn!("overlapped block's transaction does not match");
                        return BatchValidity::Drop;
                    }
                }
                let safe_block_ref = match safe_block_payload.to_l2_block_ref(cfg) {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("failed to extract L2BlockRef from execution payload, hash: {}, err: {e}", safe_block_payload.execution_payload.block_hash);
                        return BatchValidity::Drop;
                    }
                };
                if safe_block_ref.l1_origin.number != self.batches[i as usize].epoch_num {
                    warn!("overlapped block's L1 origin number does not match");
                    return BatchValidity::Drop;
                }
            }
        }

        BatchValidity::Accept
    }

    /// Converts the span batch to a raw span batch.
    pub fn to_raw_span_batch(
        &self,
        origin_changed_bit: u8,
        genesis_timestamp: u64,
        chain_id: u64,
    ) -> Result<RawSpanBatch, SpanBatchError> {
        if self.batches.is_empty() {
            return Err(SpanBatchError::EmptySpanBatch);
        }

        let span_start = self.batches.first().ok_or(SpanBatchError::EmptySpanBatch)?;
        let span_end = self.batches.last().ok_or(SpanBatchError::EmptySpanBatch)?;

        Ok(RawSpanBatch {
            prefix: SpanBatchPrefix {
                rel_timestamp: span_start.timestamp - genesis_timestamp,
                l1_origin_num: span_end.epoch_num,
                parent_check: self.parent_check,
                l1_origin_check: self.l1_origin_check,
            },
            payload: SpanBatchPayload {
                block_count: self.batches.len() as u64,
                origin_bits: self.origin_bits.clone(),
                block_tx_counts: self.block_tx_counts.clone(),
                txs: self.txs.clone(),
            },
        })
    }

    /// Converts all [SpanBatchElement]s after the L2 safe head to [SingleBatch]es. The resulting
    /// [SingleBatch]es do not contain a parent hash, as it is populated by the Batch Queue
    /// stage.
    pub fn get_singular_batches(
        &self,
        l1_origins: &[BlockInfo],
        l2_safe_head: L2BlockInfo,
    ) -> Result<Vec<SingleBatch>, SpanBatchError> {
        let mut single_batches = Vec::new();
        let mut origin_index = 0;
        for batch in &self.batches {
            if batch.timestamp <= l2_safe_head.block_info.timestamp {
                continue;
            }
            let origin_epoch_hash = l1_origins[origin_index..l1_origins.len()]
                .iter()
                .enumerate()
                .find(|(i, origin)| origin.timestamp == batch.timestamp)
                .map(|(i, origin)| {
                    origin_index = i;
                    origin.hash
                })
                .ok_or(SpanBatchError::MissingL1Origin)?;
            let mut single_batch = SingleBatch {
                epoch_num: batch.epoch_num,
                epoch_hash: origin_epoch_hash,
                timestamp: batch.timestamp,
                transactions: batch.transactions.clone(),
                ..Default::default()
            };
            single_batches.push(single_batch);
        }
        Ok(single_batches)
    }

    /// Append a [SingleBatch] to the [SpanBatch]. Updates the L1 origin check if need be.
    pub fn append_singular_batch(
        &mut self,
        singular_batch: SingleBatch,
        seq_num: u64,
    ) -> Result<(), SpanBatchError> {
        // If the new element is not ordered with respect to the last element, panic.
        if !self.batches.is_empty() && self.peek(0).timestamp > singular_batch.timestamp {
            panic!("Batch is not ordered");
        }

        let SingleBatch { epoch_hash, parent_hash, .. } = singular_batch;

        // Always append the new batch and set the L1 origin check.
        self.batches.push(singular_batch.into());
        // Always update the L1 origin check.
        self.l1_origin_check = epoch_hash[..20].try_into().expect("Sub-slice cannot fail");

        let epoch_bit = if self.batches.len() == 1 {
            // If there is only one batch, initialize the parent check and set the epoch bit based
            // on the sequence number.
            self.parent_check = parent_hash[..20].try_into().expect("Sub-slice cannot fail");
            seq_num == 0
        } else {
            // If there is more than one batch, set the epoch bit based on the last two batches.
            self.peek(1).epoch_num < self.peek(0).epoch_num
        };

        // Set the respective bit in the origin bits.
        self.origin_bits.set_bit(self.batches.len() - 1, epoch_bit);

        let new_txs = self.peek(0).transactions.clone();

        // Update the block tx counts cache with the latest batch's transaction count.
        self.block_tx_counts.push(new_txs.len() as u64);

        // Add the new transactions to the transaction cache.
        self.txs.add_txs(new_txs, self.chain_id)
    }

    /// Peek at the `n`th-to-last last element in the batch.
    fn peek(&self, n: usize) -> &SpanBatchElement {
        &self.batches[self.batches.len() - 1 - n]
    }
}
