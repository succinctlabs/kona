#!/bin/bash

pushd ../programs/zkvm-client
cargo prove build --ignore-rust-version
popd
