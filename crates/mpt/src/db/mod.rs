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

/// A Trie DB that caches account state in-memory. When accounts that don't already exist within the
/// cache are queried, the database fetches the preimages of the trie nodes on the path to the
/// account using the `PreimageFetcher` (`PF` generic) and `CodeHashFetcher` (`CHF` generic). This
/// allows for data to be fetched in a verifiable manner given an initial trusted state root as it
/// is needed during execution.
///
/// **Behavior**:
/// - When an account is queried and it does not already exist in the inner cache database, we fall
///   through to the `PreimageFetcher` to fetch the preimages of the trie nodes on the path to the
///   account. After it has been fetched, the account is inserted into the cache database and will
///   be read from there on subsequent queries.
/// - When querying for the code hash of an account, the `CodeHashFetcher` is consulted to fetch the
///   code hash of the account.
/// - When a changeset is committed to the database, the changes are first applied to the cache
///   database and then the trie hash is recomputed. The root hash of the trie is then persisted to
///   the struct.
#[derive(Debug, Clone)]
pub struct TrieCacheDB<PF> {
    /// The underlying DB that stores the account state in-memory.
    db: InMemoryDB,
    /// The current root node of the trie.
    root: B256,

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
            db: InMemoryDB::default(),
            _phantom_pf: PhantomData,
        }
    }

    /// Returns the current state root of the trie DB.
    pub fn root(&self) -> B256 {
        self.root
    }

    /// Returns a reference to the underlying in-memory DB.
    pub fn inner_db_ref(&self) -> &InMemoryDB {
        &self.db
    }

    /// Returns a mutable reference to the underlying in-memory DB.
    pub fn inner_db_mut(&mut self) -> &mut InMemoryDB {
        &mut self.db
    }

    /// Returns the account for the given address.
    ///
    /// If the  account was not found in the cache, it will be loaded from the underlying database.
    ///
    /// TODO: Check if it exists in the trie, if not, fetch it.
    pub fn load_account(&mut self, address: Address) -> Result<&mut DbAccount> {
        let db = self.inner_db_mut();
        match db.accounts.entry(address) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => Ok(entry.insert(
                db.db
                    .basic_ref(address)
                    .map_err(|e| anyhow!(e))?
                    .map(|info| DbAccount { info, ..Default::default() })
                    .unwrap_or_else(DbAccount::new_not_existing),
            )),
        }
    }

    /// Loads an account from the trie by consulting the `PreimageFetcher` to fetch the preimages of the trie nodes on 
    /// the path to the account. Once the account is reached, the storage trie is walked to hydrate the account's 
    /// storage map.
    pub fn load_account_from_trie(&self, address: Address, fetcher: PF) -> Result<DbAccount> {
        let root_node = TrieNode::decode(&mut fetcher(self.root)?.as_ref()).map_err(|e| anyhow!(e))?;

        let hashed_address_nibbles = Nibbles::unpack(keccak256(address.as_slice()));
        let trie_account_rlp = retrieve(&hashed_address_nibbles, root_node, 0, fetcher)?;
        let trie_account =
            TrieAccount::decode(&mut trie_account_rlp.as_ref()).map_err(|e| anyhow!(e))?;

        let storage = if trie_account.storage_root != EMPTY_ROOT_HASH {
            let storage_walker = OrderedListWalker::try_new_hydrated(trie_account.storage_root, fetcher)?;
            
            todo!()
        } else {
            Default::default()
        };

        Ok(DbAccount {
            info: AccountInfo {
                balance: trie_account.balance,
                nonce: trie_account.nonce,
                code_hash: trie_account.code_hash,
                code: todo!(),
            },
            account_state: Default::default(),
            storage: todo!(),
        })
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
        todo!()
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
