#!/bin/bash

export ETH_RPC_URL="http://anton.clab.by:8547"
BLOCK_NUMBER=$1

PARENT_HEADER=$(cast rpc "debug_getRawHeader" $(cast 2h $((BLOCK_NUMBER - 1))) | jq -r)
HEADER=$(cast rpc "debug_getRawHeader" $(cast 2h $BLOCK_NUMBER) | jq -r)

echo "PARENT_HEADER: $PARENT_HEADER"
echo ""
echo "HEADER: $HEADER"
echo ""

cast block $BLOCK_NUMBER -j | jq -r '.transactions[]' | while read -r tx; do
    raw_tx=$(cast rpc "debug_getRawTransaction" $tx | jq -r | cut -c3-)
    echo "hex!(\"$raw_tx\").into(),"
done
