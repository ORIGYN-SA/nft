#!/bin/bash

ulimit -n 65536

# Release build first: it produces the shipped artifacts (index_icrc7 included,
# the tests use its release wasm) and keeps them free of the test-only endpoints.
./scripts/build.sh || exit 1

# Then the core_nft build the tests actually load: same sources plus the
# `inttest` feature, written to src/core_nft/wasm/*_inttest.*.
./scripts/build.sh --inttest || exit 1

cargo test -p integration_tests -- --test-threads=1
