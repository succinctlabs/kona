//! This module contains an implementation of an in-memory Trie DB for [revm], that allows for
//! incremental updates through fetching node preimages on the fly during execution.

#![allow(dead_code, unused)]

use crate::{retrieve, NodeElement, OrderedListWalker, TrieNode};
use alloc::collections::VecDeque;
use alloy_consensus::EMPTY_ROOT_HASH;
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use alloy_rlp::Decodable;
use alloy_trie::Nibbles;
use anyhow::{anyhow, Result};
use core::marker::PhantomData;
use revm::{db::DbAccount, Database, DatabaseCommit, DatabaseRef, InMemoryDB};
use revm_primitives::{hash_map::Entry, Account, AccountInfo, Bytecode, HashMap};

mod account;
pub use account::TrieAccount;

mod cache;
pub use cache::CacheDB;

/// A non-caching in-memory trie DB, meant to underly [revm]'s [CacheDB]. When accounts that don't
/// are queried, the database fetches the preimages of the trie nodes on the path to the account
/// using the `PreimageFetcher` (`PF` generic) and `CodeHashFetcher` (`CHF` generic). This allows
/// for data to be fetched in a verifiable manner given an initial trusted state root as it
/// is needed during execution.
///
/// **Behavior**:
/// - When an account is queried, the `PreimageFetcher` is consulted to fetch the preimages of the trie
///   nodes on the path to the account.
/// - When querying for the code hash of an account, the `CodeHashFetcher` is consulted to fetch the
///   code hash of the account.
/// - When a changeset is committed to the database, the changes are first applied to the bundle
///   and then the trie hash is recomputed at any point w/ [placeholder]. The root hash of the trie
///   is then persisted to the struct.
#[derive(Debug, Clone)]
pub struct TrieCacheDB<PF> {
    /// The current root node of the trie.
    root: B256,
    /// Account info where None means it either does not exist or has not been loaded from the trie yet.
    accounts: HashMap<Address, DbAccount>,
    /// Tracks all contracts by their code hash.
    contracts: HashMap<B256, Bytecode>,
    /// All cached block hashes.
    block_hashes: HashMap<U256, B256>,

    _phantom_pf: PhantomData<PF>,
}

impl<PF> TrieCacheDB<PF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
{
    /// Creates a new [TrieCacheDB] with the given root node.
    pub fn new(root: B256) -> Self {
        Self {
            root,
            _phantom_pf: PhantomData,
            accounts: Default::default(),
            contracts: Default::default(),
            block_hashes: Default::default(),
        }
    }

    /// Returns the current state root of the trie DB.
    pub fn root(&self) -> B256 {
        self.root
    }

    /// Loads a [TrieAccount] from the root of the trie by querying for the passed [Address].
    pub fn load_trie_account(&self, address: Address, fetcher: PF) -> Result<TrieAccount> {
        let root_node =
            TrieNode::decode(&mut fetcher(self.root)?.as_ref()).map_err(|e| anyhow!(e))?;

        let hashed_address_nibbles = Nibbles::unpack(keccak256(address.as_slice()));
        let trie_account_rlp = retrieve(&hashed_address_nibbles, root_node, 0, fetcher)?;
        TrieAccount::decode(&mut trie_account_rlp.as_ref()).map_err(|e| anyhow!(e))
    }

    /// Loads an account from the trie by consulting the `PreimageFetcher` to fetch the preimages of the trie nodes on
    /// the path to the account.
    pub fn load_account_from_trie(&self, address: Address, fetcher: PF) -> Result<DbAccount> {
        let trie_account = self.load_trie_account(address, fetcher)?;

        Ok(DbAccount {
            info: AccountInfo {
                balance: trie_account.balance,
                nonce: trie_account.nonce,
                code_hash: trie_account.code_hash,
                code: None,
            },
            account_state: Default::default(),
            storage: Default::default(),
        })
    }

    /// Loads a storage slot value from an account's storage trie by consulting the `PreimageFetcher` to fetch the
    /// preimages of the trie nodes on the path to the slot's leaf. This function should be used if the [TrieAccount]
    /// is not already loaded and not otherwise needed.
    pub fn load_account_storage_slot(
        &self,
        address: Address,
        slot_key: B256,
        fetcher: PF,
    ) -> Result<B256> {
        let trie_account = self.load_trie_account(address, fetcher)?;
        self.load_storage_slot(trie_account.storage_root, slot_key, fetcher)
    }

    /// Loads a storage slot value from an account's storage trie by consulting the `PreimageFetcher` to fetch the
    /// preimages of the trie nodes on the path to the slot's leaf.
    fn load_storage_slot(
        &self,
        account_storage_root: B256,
        slot_key: B256,
        fetcher: PF,
    ) -> Result<B256> {
        let root_node = TrieNode::decode(&mut fetcher(account_storage_root)?.as_ref())
            .map_err(|e| anyhow!(e))?;

        let hashed_slot_key_nibbles = Nibbles::unpack(keccak256(slot_key.as_slice()));
        let slot_value_rlp = retrieve(&hashed_slot_key_nibbles, root_node, 0, fetcher)?;
        let slot_value = B256::try_from(slot_value_rlp.as_ref()).map_err(|e| anyhow!(e))?;

        Ok(slot_value)
    }
}

impl<PF> DatabaseCommit for TrieCacheDB<PF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
{
    fn commit(&mut self, changes: HashMap<Address, Account>) {
        todo!()
    }
}

impl<PF> Database for TrieCacheDB<PF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
{
    type Error = anyhow::Error;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        todo!()
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        todo!()
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        match self.inner_db_mut().storage(address, index) {
            Err(e) => {
                // Assume that the storage key
            }
            res => res.map_err(|e| anyhow!(e)),
        }
    }

    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
        todo!()
    }
}

impl<PF> DatabaseRef for TrieCacheDB<PF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
{
    type Error = anyhow::Error;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        todo!()
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        todo!()
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        todo!()
    }

    fn block_hash_ref(&self, number: U256) -> Result<B256, Self::Error> {
        todo!()
    }
}
