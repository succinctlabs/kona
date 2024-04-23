//! Contains the [retrieve] function, allowing for retrieving values from leaves within a Merkle
//! Patricia Trie by key.

use crate::{NodeElement, TrieNode};
use alloy_primitives::{Bytes, B256};
use alloy_rlp::Decodable;
use alloy_trie::Nibbles;
use anyhow::{anyhow, Result};

/// Walks down the trie to a leaf value with the given key, if it exists. Preimages for blinded
/// nodes along the path are fetched using the `fetcher` function.
///
/// ## Takes
/// - `item_key` - The nibbles representation of the key being retrieved
/// - `trie_node` - The root trie node
/// - `pos` - The number of nibbles that have already been traversed in the `item_key`
/// - `fetcher` - The preimage fetcher for intermediate blinded nodes
///
/// ## Returns
/// - `Err(_)` - Could not retrieve the node with the given key from the trie.
/// - `Ok((_, _))` - The key and value of the node
pub fn retrieve(
    item_key: &Nibbles,
    trie_node: TrieNode,
    mut pos: usize,
    fetcher: impl Fn(B256) -> Result<Bytes> + Copy,
) -> Result<Bytes> {
    match trie_node {
        TrieNode::Branch { mut stack } => {
            let branch_nibble = item_key[pos];
            pos += 1;

            match stack
                .remove(branch_nibble as usize)
                .ok_or(anyhow!("Key does not exist in trie"))?
            {
                NodeElement::String(s) => {
                    // If the string is a hash, we need to grab the preimage for it and
                    // continue recursing.
                    let hash: B256 =
                        s.as_ref().try_into().map_err(|e| anyhow!("Conversion error: {e}"))?;
                    let trie_node =
                        TrieNode::decode(&mut fetcher(hash)?.as_ref()).map_err(|e| anyhow!(e))?;

                    // If the value was found in the blinded node, return it.
                    if let Ok(value) = retrieve(item_key, trie_node, pos, fetcher) {
                        return Ok(value);
                    }
                }
                list @ NodeElement::List(_) => {
                    let trie_node = list.try_list_into_node()?;

                    // If the value was found in the blinded node, return it.
                    if let Ok(value) = retrieve(item_key, trie_node, pos, fetcher) {
                        return Ok(value);
                    }
                }
                _ => { /* Skip over empty lists and strings; We're looking for leaves */ }
            };

            anyhow::bail!("Key does not exist in trie");
        }
        TrieNode::Leaf { key, value } => {
            // If the key length is one, it only contains the prefix and no shared nibbles. Return
            // the key and value.
            if key.len() == 1 {
                return Ok(value);
            }

            let key_nibbles = Nibbles::unpack(key.clone());
            let shared_nibbles = key_nibbles[1..].as_ref();
            let item_key_nibbles = item_key[pos..pos + shared_nibbles.len()].as_ref();

            if item_key_nibbles == shared_nibbles {
                Ok(value)
            } else {
                anyhow::bail!("Key does not exist in trie");
            }
        }
        TrieNode::Extension { prefix, node } => {
            let prefix_nibbles = Nibbles::unpack(prefix);
            let shared_nibbles = prefix_nibbles[1..].as_ref();
            let item_key_nibbles = item_key[pos..pos + shared_nibbles.len()].as_ref();
            if item_key_nibbles == shared_nibbles {
                // Increase the offset within the key by the length of the shared nibbles
                pos += shared_nibbles.len();

                // Follow extension branch
                let hash = B256::from_slice(node.as_ref());
                let extension_link =
                    TrieNode::decode(&mut fetcher(hash)?.as_ref()).map_err(|e| anyhow!(e))?;
                retrieve(item_key, extension_link, pos, fetcher)
            } else {
                anyhow::bail!("Key does not exist in trie");
            }
        }
    }
}

#[cfg(test)]
mod test {
    use alloc::{collections::BTreeMap, vec::Vec};
    use alloy_primitives::{b256, keccak256, Bytes, B256};
    use alloy_provider::{Provider, ProviderBuilder};
    use alloy_rlp::{Decodable, Encodable, EMPTY_STRING_CODE};
    use alloy_trie::Nibbles;
    use anyhow::{anyhow, Result};
    use reqwest::Url;
    use tokio::runtime::Runtime;

    use crate::{retrieve, test_util::ordered_trie_with_encoder, TrieNode};

    #[test]
    fn test_retrieve_from_trie_simple() {
        const VALUES: [&str; 5] = ["yeah", "dog", ", ", "laminar", "flow"];

        let mut trie = ordered_trie_with_encoder(&VALUES, |v, buf| v.encode(buf));
        let root = trie.root();

        let preimages =
            trie.take_proofs().into_iter().fold(BTreeMap::default(), |mut acc, (_, value)| {
                acc.insert(keccak256(value.as_ref()), value);
                acc
            });
        let fetcher = |h: B256| -> Result<Bytes> {
            preimages.get(&h).cloned().ok_or(anyhow!("Failed to find preimage"))
        };

        let root = TrieNode::decode(&mut fetcher(root).unwrap().as_ref()).unwrap();

        for (i, value) in VALUES.iter().enumerate() {
            let key_nibbles = Nibbles::unpack([if i == 0 { EMPTY_STRING_CODE } else { i as u8 }]);
            let v = retrieve(&key_nibbles, root.clone(), 0, fetcher).unwrap();

            let mut encoded_value = Vec::with_capacity(value.length());
            value.encode(&mut encoded_value);

            assert_eq!(v, encoded_value);
        }
    }

    fn test_online_retrieval() {
        extern crate std;
        use std::dbg;

        const RPC_URL: &str = "https://mainnet-replica-0-op-geth.primary.client.dev.oplabs.cloud";

        // Initialize the provider.
        let provider = ProviderBuilder::new()
            .on_http(Url::parse(RPC_URL).expect("invalid rpc url"))
            .map_err(|e| anyhow!(e))
            .unwrap();

        let block_number = 19005266;
        let block = futures::executor::block_on(async { provider.get_block(block_number.into(), true) }).await.unwrap().unwrap();

        let root = block.header.state_root;
        let fetch = |hash: B256| -> Result<Bytes> {
            let preimage = futures::executor::block_on(async {
                provider
                    .client()
                    .request::<&[B256; 1], Bytes>("debug_dbGet", &[hash])
                    .await
                    .unwrap()
            });
            Ok(preimage)
        };

        dbg!(fetch(b256!("7cb368272bf53782d801c81c5a326202d21a5a5c6cbf292c7223838301acfbf3"))
            .unwrap());
    }
}
