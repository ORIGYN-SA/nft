#!/bin/bash

BASE_CANISTER_PATH=$1
CANISTER=$2

cargo rustc --crate-type=cdylib --target wasm32-unknown-unknown --target-dir "$BASE_CANISTER_PATH/$CANISTER/target" --release --locked -p $CANISTER &&
ic-wasm "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/$CANISTER.wasm" -o "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/$CANISTER.wasm" shrink &&
ic-wasm "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/$CANISTER.wasm" -o "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/$CANISTER.wasm" optimize --inline-functions-with-loops O3 &&
gzip --no-name -9 -v -c "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/$CANISTER.wasm" > "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/${CANISTER}_canister.wasm.gz" &&
gzip -v -t "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/${CANISTER}_canister.wasm.gz" &&
mv "$BASE_CANISTER_PATH/$CANISTER/target/wasm32-unknown-unknown/release/${CANISTER}_canister.wasm.gz" "$BASE_CANISTER_PATH/$CANISTER/wasm/${CANISTER}_canister.wasm.gz"
