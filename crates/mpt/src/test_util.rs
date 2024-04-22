//! Testing utilities for `kona-mpt`

extern crate std;

use std::dbg;

use alloc::{
    collections::BTreeMap,
    vec::{self, Vec},
};
use alloy_consensus::{Receipt, ReceiptEnvelope, ReceiptWithBloom, TxEnvelope, TxType};
use alloy_primitives::{b256, keccak256, Bytes, Log, B256};
use alloy_provider::{network::eip2718::Encodable2718, Provider, ProviderBuilder};
use alloy_rlp::{BufMut, Decodable, Encodable};
use alloy_rpc_types::BlockTransactions;
use alloy_trie::{HashBuilder, Nibbles};
use anyhow::{anyhow, Result};
use reqwest::Url;

use crate::{NodeElement, TrieNode};

const RPC_URL: &str = "https://docs-demo.quiknode.pro/";

/// Grabs a live merkleized receipts list within a block header.
pub(crate) async fn get_live_derivable_receipts_list(
) -> Result<(B256, BTreeMap<B256, Bytes>, Vec<ReceiptEnvelope>)> {
    // Initialize the provider.
    let provider = ProviderBuilder::new()
        .on_http(Url::parse(RPC_URL).expect("invalid rpc url"))
        .map_err(|e| anyhow!(e))?;

    let block_number = 19005266;
    let block = provider
        .get_block(block_number.into(), true)
        .await
        .map_err(|e| anyhow!(e))?
        .ok_or(anyhow!("Missing block"))?;
    let receipts = provider
        .get_block_receipts(block_number.into())
        .await
        .map_err(|e| anyhow!(e))?
        .ok_or(anyhow!("Missing receipts"))?;

    let consensus_receipts = receipts
        .into_iter()
        .map(|r| {
            let rpc_receipt = r.inner.as_receipt_with_bloom().expect("Infalliable");
            let consensus_receipt = ReceiptWithBloom::new(
                Receipt {
                    status: rpc_receipt.receipt.status,
                    cumulative_gas_used: rpc_receipt.receipt.cumulative_gas_used,
                    logs: rpc_receipt
                        .receipt
                        .logs
                        .iter()
                        .map(|l| Log { address: l.address(), data: l.data().clone() })
                        .collect(),
                },
                rpc_receipt.logs_bloom,
            );

            match r.transaction_type() {
                TxType::Legacy => ReceiptEnvelope::Legacy(consensus_receipt),
                TxType::Eip2930 => ReceiptEnvelope::Eip2930(consensus_receipt),
                TxType::Eip1559 => ReceiptEnvelope::Eip1559(consensus_receipt),
                TxType::Eip4844 => ReceiptEnvelope::Eip4844(consensus_receipt),
            }
        })
        .collect::<Vec<_>>();

    // Compute the derivable list
    let mut list =
        ordered_trie_with_encoder(consensus_receipts.as_ref(), |rlp, buf| rlp.encode_2718(buf));
    let root = list.root();

    // Sanity check receipts root is correct
    assert_eq!(block.header.receipts_root, root);

    // Construct the mapping of hashed intermediates -> raw intermediates
    let preimages =
        list.take_proofs().into_iter().fold(BTreeMap::default(), |mut acc, (_, value)| {
            acc.insert(keccak256(value.as_ref()), value);
            acc
        });

    Ok((root, preimages, consensus_receipts))
}

/// Grabs a live merkleized transactions list within a block header.
pub(crate) async fn get_live_derivable_transactions_list(
) -> Result<(B256, BTreeMap<B256, Bytes>, Vec<TxEnvelope>)> {
    // Initialize the provider.
    let provider = ProviderBuilder::new()
        .on_http(Url::parse(RPC_URL).expect("invalid rpc url"))
        .map_err(|e| anyhow!(e))?;

    let block_number = 19005266;
    let block = provider
        .get_block(block_number.into(), true)
        .await
        .map_err(|e| anyhow!(e))?
        .ok_or(anyhow!("Missing block"))?;

    let BlockTransactions::Full(txs) = block.transactions else {
        anyhow::bail!("Did not fetch full block");
    };
    let consensus_txs = txs
        .into_iter()
        .map(|tx| TxEnvelope::try_from(tx).map_err(|e| anyhow!(e)))
        .collect::<Result<Vec<_>>>()?;

    // Compute the derivable list
    let mut list =
        ordered_trie_with_encoder(consensus_txs.as_ref(), |rlp, buf| rlp.encode_2718(buf));
    let root = list.root();

    // Sanity check transaction root is correct
    assert_eq!(block.header.transactions_root, root);

    // Construct the mapping of hashed intermediates -> raw intermediates
    let preimages =
        list.take_proofs().into_iter().fold(BTreeMap::default(), |mut acc, (_, value)| {
            acc.insert(keccak256(value.as_ref()), value);
            acc
        });

    Ok((root, preimages, consensus_txs))
}

/// Compute a trie root of the collection of items with a custom encoder.
pub(crate) fn ordered_trie_with_encoder<T, F>(items: &[T], mut encode: F) -> HashBuilder
where
    F: FnMut(&T, &mut dyn BufMut),
{
    let mut index_buffer = Vec::new();
    let mut value_buffer = Vec::new();
    let items_len = items.len();

    // Store preimages for all intermediates
    let path_nibbles = (0..items_len)
        .map(|i| {
            let i = adjust_index_for_rlp(i, items_len);
            index_buffer.clear();
            i.encode(&mut index_buffer);
            Nibbles::unpack(&index_buffer)
        })
        .collect::<Vec<_>>();

    let mut hb = HashBuilder::default().with_proof_retainer(path_nibbles);
    for i in 0..items_len {
        let index = adjust_index_for_rlp(i, items_len);

        index_buffer.clear();
        index.encode(&mut index_buffer);

        value_buffer.clear();
        encode(&items[index], &mut value_buffer);

        hb.add_leaf(Nibbles::unpack(&index_buffer), &value_buffer);
    }

    hb
}

/// Adjust the index of an item for rlp encoding.
pub(crate) const fn adjust_index_for_rlp(i: usize, len: usize) -> usize {
    if i > 0x7f {
        i
    } else if i == 0x7f || i + 1 == len {
        0
    } else {
        i + 1
    }
}

#[test]
fn test_trie() {
    use alloc::vec;

    let mut hb = HashBuilder::default().with_proof_retainer(vec![
        Nibbles::unpack(&[0x80]),
        Nibbles::unpack(&[0x01]),
        Nibbles::unpack(&[0xFF]),
    ]);

    hb.add_leaf(Nibbles::unpack(&[0x01]), b"test two");
    hb.add_leaf(Nibbles::unpack(&[0x88]), b"test one");
    hb.add_leaf(Nibbles::unpack(&[0xFF]), b"test three");
    // hb.add_branch(
    //     Nibbles::unpack(&[0x00]),
    //     b256!("f4ae7801fd7296c9cb9f2387149e93079bd7c74158fea76d978947fddbead8b7"),
    //     true,
    // );
    // hb.add_branch(
    //     Nibbles::unpack(&[0x01]),
    //     b256!("91c0bc2b7771df00372f3b3ec799e2586115046fabc2b406c94b4d793ff1669c"),
    //     true,
    // );
    std::dbg!(hb.root());

    let proofs = hb.take_proofs();
    let preimages = proofs.into_iter().fold(BTreeMap::default(), |mut acc, (_, v)| {
        acc.insert(keccak256(v.as_ref()), v);
        acc
    });
    let fetcher = |hash: B256| -> Result<Bytes> { Ok(preimages.get(&hash).cloned().unwrap()) };

    let root = TrieNode::decode(&mut fetcher(hb.root()).unwrap().as_ref()).unwrap();
    std::dbg!(get_trie(&Nibbles::unpack(&[0x01]), root, 0, fetcher).unwrap());
}

fn adjust_index_for_read(index: usize) -> usize {
    match index.cmp(&0x80) {
        core::cmp::Ordering::Less => index,
        core::cmp::Ordering::Equal => 0x00,
        core::cmp::Ordering::Greater => index - 1,
    }
}

#[tokio::test]
async fn test_trie_get() {
    // Initialize the provider.
    let provider =
        ProviderBuilder::new().on_http(Url::parse(RPC_URL).expect("invalid rpc url")).unwrap();

    let block_number = 19005266;
    let block = provider.get_block(block_number.into(), true).await.unwrap().unwrap();

    let BlockTransactions::Full(txs) = block.transactions else {
        panic!("Did not fetch full block");
    };
    let consensus_txs = txs
        .into_iter()
        .map(|tx| TxEnvelope::try_from(tx).map_err(|e| anyhow!(e)))
        .collect::<Result<Vec<_>>>()
        .unwrap();

    // Compute the derivable list
    let mut list =
        ordered_trie_with_encoder(consensus_txs.as_ref(), |rlp, buf| rlp.encode_2718(buf));
    let root = list.root();

    // Sanity check transaction root is correct
    assert_eq!(block.header.transactions_root, root);

    // Construct the mapping of hashed intermediates -> raw intermediates
    let proofs = list.take_proofs();
    let preimages = proofs.into_iter().fold(BTreeMap::default(), |mut acc, (_, value)| {
        acc.insert(keccak256(value.as_ref()), value);
        acc
    });

    let fetcher = |hash: B256| -> Result<Bytes> { Ok(preimages.get(&hash).cloned().unwrap()) };

    let root = TrieNode::decode(&mut fetcher(root).unwrap().as_ref()).unwrap();
    for i in 1..135 {
        let (_, v) = get_trie(
            &Nibbles::unpack(alloc::vec![i as u8]),
            root.clone(),
            0,
            fetcher,
        )
        .unwrap();
        let mut rlp_buf = Vec::new();
        consensus_txs[adjust_index_for_read(i)].encode_2718(&mut rlp_buf);
        assert_eq!(v.as_ref(), rlp_buf.as_slice(), "Failed at index: {}", i);
    }
    dbg!(block.header.number);
    dbg!(consensus_txs[0x0].tx_hash());
}

/// Walks down the trie to a leaf value with the given key, if it exists. Preimages for blinded nodes along the
/// path are fetched using the `fetcher` function.
fn get_trie(
    item_key: &Nibbles,
    trie_node: TrieNode,
    mut pos: usize,
    fetcher: impl Fn(B256) -> Result<Bytes> + Copy,
) -> Result<(Bytes, Bytes)> {
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
                    if let Ok((key, value)) = get_trie(item_key, trie_node, pos, fetcher) {
                        return Ok((key, value));
                    }
                }
                list @ NodeElement::List(_) => {
                    let trie_node = list.try_list_into_node()?;

                    // If the value was found in the blinded node, return it.
                    if let Ok((key, value)) = get_trie(item_key, trie_node, pos, fetcher) {
                        return Ok((key, value));
                    }
                }
                _ => { /* Skip over empty lists and strings; We're looking for leaves */ }
            };

            anyhow::bail!("Key does not exist in trie");
        }
        TrieNode::Leaf { key, value } => {
            // If the key length is one, it only contains the prefix and no shared nibbles. Return the
            // key and value.
            if key.len() == 1 {
                return Ok((key, value));
            }

            let key_nibbles = Nibbles::unpack(key.clone());
            let shared_nibbles = key_nibbles[1..].as_ref();
            let item_key_nibbles = item_key[pos..pos + shared_nibbles.len()].as_ref();

            if item_key_nibbles == shared_nibbles {
                Ok((key, value))
            } else {
                anyhow::bail!("Key does not exist in trie");
            }
        }
        TrieNode::Extension { prefix, node } => {
            std::dbg!(&prefix);
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
                get_trie(item_key, extension_link, pos, fetcher)
            } else {
                anyhow::bail!("Key does not exist in trie");
            }
        }
    }
}
