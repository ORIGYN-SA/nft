#!/bin/bash

ulimit -n 65536

./scripts/build.sh

mkdir -p integrations_tests/wasm
cp src/core_nft/wasm/core_nft_canister.wasm.gz integrations_tests/wasm/

cargo test -p integration_tests -- --test-threads=1