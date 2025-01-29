#!/bin/bash

BASE_CANISTER_PATH="./src"

# Build storage canister
./scripts/build.sh "$BASE_CANISTER_PATH" "storage_canister"

# Build core <- need storage canister to be built first
./scripts/build.sh "$BASE_CANISTER_PATH" "core"
