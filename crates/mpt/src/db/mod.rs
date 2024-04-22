//! This module contains an implementation of an in-memory Trie DB, that allows for incremental updates through fetching
//! node preimages on the fly.

#![allow(dead_code, unused)]

use alloc::collections::VecDeque;
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use alloy_rlp::Decodable;
use alloy_trie::Nibbles;
use anyhow::{anyhow, Result};
use core::marker::PhantomData;
use revm::{db::DbAccount, Database, DatabaseCommit, DatabaseRef, InMemoryDB};
use revm_primitives::{hash_map::Entry, Account, AccountInfo, Bytecode, HashMap};

use crate::{NodeElement, TrieNode};

/// A Trie DB that caches account state in-memory. When accounts that don't already exist within the cache are queried,
/// the database fetches the preimages of the trie nodes on the path to the account using the `PreimageFetcher`
/// (`PF` generic) and `CodeHashFetcher` (`CHF` generic). This allows for data to be fetched in a verifiable manner
/// given an initial trusted state root as it is needed during execution.
///
/// **Behavior**:
/// - When an account is queried and it does not already exist in the inner cache database, we fall through to the
///   `PreimageFetcher` to fetch the preimages of the trie nodes on the path to the account. After it has been fetched,
///   the account is inserted into the cache database and will be read from there on subsequent queries.
/// - When querying for the code hash of an account, the `CodeHashFetcher` is consulted to fetch the code hash of the
///   account.
/// - When a changeset is committed to the database, the changes are first applied to the cache database and then the
///   trie is recomputed. The root hash of the trie is then persisted as
#[derive(Debug, Default, Clone)]
pub struct TrieCacheDB<PF, CHF> {
    /// The underlying DB that stores the account state in-memory.
    db: InMemoryDB,
    /// The current root node of the trie.
    root: B256,

    _phantom_pf: PhantomData<PF>,
    _phantom_chf: PhantomData<CHF>,
}

impl<PF, CHF> TrieCacheDB<PF, CHF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy + Default,
    CHF: Fn(Address) -> Result<Bytes> + Copy + Default,
{
    /// Creates a new [TrieCacheDB] with the given root node.
    pub fn new(root: B256) -> Self {
        Self { root, ..Default::default() }
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

    fn get_account_trie(&self, address: Address, fetcher: PF) -> Result<Account> {
        let key = keccak256(address.as_slice());
        let root_node = TrieNode::decode(&mut fetcher(key)?.as_ref());

        todo!()
    }

    /// Walks down the trie to a leaf value with the given key, if it exists. Preimages for blinded nodes along the
    /// path are fetched using the `fetcher` function.
    ///
    /// TODO: Fix nibble relations
    fn get_trie(
        &self,
        item_key: &Bytes,
        trie_node: TrieNode,
        mut pos: usize,
        fetcher: PF,
    ) -> Result<(Bytes, Bytes)> {
        match trie_node {
            TrieNode::Branch { stack } => {
                let next = item_key[pos];
                extern crate std;
                std::dbg!(next);

                // for (i, node) in stack.into_iter().enumerate() {
                //     match node {
                //         NodeElement::String(s) => {
                //             // If the string is a hash, we need to grab the preimage for it and
                //             // continue recursing.
                //             let hash: B256 = s.as_ref().try_into().map_err(|e| anyhow!("Conversion error: {e}"))?;
                //             let trie_node = TrieNode::decode(&mut fetcher(hash)?.as_ref()).map_err(|e| anyhow!(e))?;
                //
                //             // If the value was found in the blinded node, return it.
                //             if let Ok((key, value)) = self.get_trie(item_key, trie_node, pos, fetcher) {
                //                 return Ok((key, value));
                //             }
                //         }
                //         list @ NodeElement::List(_) => {
                //             let trie_node = list.try_list_into_node()?;
                //
                //             // If the value was found in the blinded node, return it.
                //             if let Ok((key, value)) = self.get_trie(item_key, trie_node, pos, fetcher) {
                //                 return Ok((key, value));
                //             }
                //         }
                //         _ => { /* Skip over empty lists and strings; We're looking for leaves */ }
                //     };
                // }

                anyhow::bail!("Key does not exist in trie");
            }
            TrieNode::Leaf { key, value } => {
                let shared_nibbles = key[1..].as_ref();
                let item_key_nibbles = item_key[pos..pos + shared_nibbles.len()].as_ref();
                if item_key_nibbles == shared_nibbles {
                    Ok((key, value))
                } else {
                    anyhow::bail!("Key does not exist in trie");
                }
            }
            TrieNode::Extension { prefix, node } => {
                let shared_nibbles = prefix[1..].as_ref();
                let item_key_nibbles = item_key[pos..pos + shared_nibbles.len()].as_ref();
                if item_key_nibbles == shared_nibbles {
                    // Increase the offset within the key by the length of the shared nibbles
                    pos += shared_nibbles.len();

                    // Follow extension branch
                    let hash = B256::from_slice(node.as_ref());
                    let extension_link =
                        TrieNode::decode(&mut fetcher(hash)?.as_ref()).map_err(|e| anyhow!(e))?;
                    self.get_trie(item_key, extension_link, pos, fetcher)
                } else {
                    anyhow::bail!("Key does not exist in trie");
                }
            }
        }
    }
}

impl<PF, CHF> DatabaseCommit for TrieCacheDB<PF, CHF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
    CHF: Fn(Address) -> Result<Bytes> + Copy,
{
    fn commit(&mut self, changes: HashMap<Address, Account>) {
        todo!()
    }
}

impl<PF, CHF> Database for TrieCacheDB<PF, CHF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
    CHF: Fn(Address) -> Result<Bytes> + Copy,
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

impl<PF, CHF> DatabaseRef for TrieCacheDB<PF, CHF>
where
    PF: Fn(B256) -> Result<Bytes> + Copy,
    CHF: Fn(Address) -> Result<Bytes> + Copy,
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
