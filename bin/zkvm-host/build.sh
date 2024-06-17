#!/bin/bash

pushd ../programs/sp1
cargo prove build --ignore-rust-version
popd
