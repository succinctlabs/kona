//! This module contains the [TrieAccount] struct.

use alloy_consensus::constants::KECCAK_EMPTY;
use alloy_primitives::{B256, U256};
use alloy_rlp::{RlpDecodable, RlpEncodable};
use revm_primitives::{Account, AccountInfo};

/// An Ethereum account as represented in the trie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, RlpEncodable, RlpDecodable)]
pub struct TrieAccount {
    /// Account nonce.
    nonce: u64,
    /// Account balance.
    balance: U256,
    /// Account's storage root.
    storage_root: B256,
    /// Hash of the account's bytecode.
    code_hash: B256,
}

impl From<(Account, B256)> for TrieAccount {
    fn from((account, storage_root): (Account, B256)) -> Self {
        Self {
            nonce: account.info.nonce,
            balance: account.info.balance,
            storage_root,
            code_hash: account.info.code_hash,
        }
    }
}

impl From<(AccountInfo, B256)> for TrieAccount {
    fn from((account, storage_root): (AccountInfo, B256)) -> Self {
        Self {
            nonce: account.nonce,
            balance: account.balance,
            storage_root,
            code_hash: account.code_hash,
        }
    }
}

impl TrieAccount {
    /// Get account's storage root.
    pub fn storage_root(&self) -> B256 {
        self.storage_root
    }
}
