#!/bin/bash

BASE_CANISTER_PATH="./src"
CANISTERS=("core_nft_impl" "index_icrc7_impl")
CANISTER_NAMES=("core_nft" "index_icrc7")

mkdir -p "./wasm"
curl -L -o "./wasm/storage_canister.wasm" "https://github.com/BitySA/ic-storage-canister/releases/latest/download/storage_canister.wasm"
curl -L -o "./wasm/storage_canister.wasm.gz" "https://github.com/BitySA/ic-storage-canister/releases/latest/download/storage_canister.wasm.gz"

# Build each canister
for i in "${!CANISTERS[@]}"; do
    CANISTER="${CANISTERS[$i]}"
    CANISTER_NAME="${CANISTER_NAMES[$i]}"
    echo "Building canister: $CANISTER"

    # Define paths variables to make the script readable and less error-prone
    TARGET_WASM="$BASE_CANISTER_PATH/$CANISTER_NAME/impl/target/wasm32-unknown-unknown/release/$CANISTER.wasm"
    FINAL_WASM_DIR="$BASE_CANISTER_PATH/$CANISTER_NAME/wasm"
    FINAL_WASM="$FINAL_WASM_DIR/$CANISTER_NAME.wasm"
    DID_FILE="$FINAL_WASM_DIR/can.did"
    FINAL_GZIP="$FINAL_WASM_DIR/${CANISTER_NAME}_canister.wasm.gz"

    # Ensure destination directory exists
    mkdir -p "$FINAL_WASM_DIR"

    # 1. Compile
    cargo rustc --crate-type=cdylib --target wasm32-unknown-unknown --target-dir "$BASE_CANISTER_PATH/$CANISTER_NAME/impl/target" --release --locked -p $CANISTER &&
    
    # 2. Shrink & Optimize
    ic-wasm "$TARGET_WASM" -o "$TARGET_WASM" shrink &&
    ic-wasm "$TARGET_WASM" -o "$TARGET_WASM" optimize --inline-functions-with-loops O3 &&

    # 3. Move optimized Wasm to final folder
    cp "$TARGET_WASM" "$FINAL_WASM" &&

    # 4. Extract Candid from the optimized Wasm
    candid-extractor "$FINAL_WASM" > "$DID_FILE" &&

    # 5. [NEW] Embed the Candid into the Wasm
    echo "Embedding candid metadata into $CANISTER..." &&
    ic-wasm "$FINAL_WASM" -o "$FINAL_WASM" metadata candid:service -f "$DID_FILE" -v public &&

    # 6. Gzip the Wasm (Now includes the metadata)
    gzip --no-name -9 -v -c "$FINAL_WASM" > "$FINAL_GZIP" &&
    gzip -v -t "$FINAL_GZIP" &&

    # 7. Copy to integration tests
    cp "$FINAL_GZIP" "./integrations_tests/wasm"
    
    echo "Finished building canister: $CANISTER"
done

echo "All canisters built successfully!"
