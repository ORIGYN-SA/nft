#!/bin/bash

BASE_CANISTER_PATH="./src"
CANISTERS=("core_nft" "index_icrc7")

mkdir -p "./wasm"
curl -L -o "./wasm/storage_canister.wasm" "https://github.com/BitySA/ic-storage-canister/releases/latest/download/storage_canister.wasm"
curl -L -o "./wasm/storage_canister.wasm.gz" "https://github.com/BitySA/ic-storage-canister/releases/latest/download/storage_canister.wasm.gz"

# Build each canister
for CANISTER in "${CANISTERS[@]}"; do
    echo "Building canister: $CANISTER"
    
    cargo rustc --crate-type=cdylib --target wasm32-unknown-unknown --target-dir "$BASE_CANISTER_PATH/$CANISTER/target" --release --locked -p $CANISTER &&
    ic-wasm "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/$CANISTER.wasm" -o "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/$CANISTER.wasm" shrink &&
    ic-wasm "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/$CANISTER.wasm" -o "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/$CANISTER.wasm" optimize --inline-functions-with-loops O3 &&
    gzip --no-name -9 -v -c "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/$CANISTER.wasm" > "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/${CANISTER}_canister.wasm.gz" &&
    gzip -v -t "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/${CANISTER}_canister.wasm.gz" &&
    cp "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/$CANISTER.wasm" "$BASE_CANISTER_PATH/$CANISTER/wasm/$CANISTER.wasm" &&
    candid-extractor "$BASE_CANISTER_PATH/$CANISTER/wasm/$CANISTER.wasm" > "$BASE_CANISTER_PATH/$CANISTER/wasm/can.did" &&
    cp "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/${CANISTER}_canister.wasm.gz" "./integrations_tests/wasm" &&
    mv "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/${CANISTER}_canister.wasm.gz" "$BASE_CANISTER_PATH/$CANISTER/wasm/${CANISTER}_canister.wasm.gz"
    
    echo "Finished building canister: $CANISTER"
done

echo "All canisters built successfully!"
