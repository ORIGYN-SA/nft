#!/bin/bash

# Usage: ./scripts/build.sh [--inttest]
#
# Default (no flag): release build. The artifacts under src/<canister>/wasm/ are
# what we publish and what other repos embed, so they must never contain the
# test-only endpoints gated behind the `inttest` cargo feature
# (__get_public_entry_test, __get_private_entry_test, __get_premint_entry_test).
#
# --inttest: builds core_nft with `--features inttest` into the git-ignored
# src/core_nft/wasm/*_inttest.* paths and touches nothing else. The integration
# tests load that wasm; the release artifacts are left untouched. index_icrc7
# has no inttest-gated code, so it is never built this way and the tests use its
# release artifact.

BASE_CANISTER_PATH="./src"

INTTEST=false
for arg in "$@"; do
    case "$arg" in
        --inttest) INTTEST=true ;;
        *)
            echo "Unknown argument: $arg" >&2
            echo "Usage: $0 [--inttest]" >&2
            exit 1
            ;;
    esac
done

if [ "$INTTEST" = true ]; then
    CANISTERS=("core_nft_impl")
    CANISTER_NAMES=("core_nft")
    FEATURE_ARGS=(--features inttest)
    SUFFIX="_inttest"
else
    CANISTERS=("core_nft_impl" "index_icrc7_impl")
    CANISTER_NAMES=("core_nft" "index_icrc7")
    FEATURE_ARGS=()
    SUFFIX=""
fi

mkdir -p "./wasm"
curl -L -o "./wasm/storage_canister.wasm" "https://github.com/BitySA/ic-storage-canister/releases/latest/download/storage_canister.wasm"
curl -L -o "./wasm/storage_canister.wasm.gz" "https://github.com/BitySA/ic-storage-canister/releases/latest/download/storage_canister.wasm.gz"

FAILED=()

# Build each canister
for i in "${!CANISTERS[@]}"; do
    CANISTER="${CANISTERS[$i]}"
    CANISTER_NAME="${CANISTER_NAMES[$i]}"

    echo "Building canister: $CANISTER${SUFFIX:+ (inttest)}"

    # Define paths variables to make the script readable and less error-prone
    TARGET_WASM="$BASE_CANISTER_PATH/$CANISTER_NAME/impl/target/wasm32-unknown-unknown/release/$CANISTER.wasm"
    FINAL_WASM_DIR="$BASE_CANISTER_PATH/$CANISTER_NAME/wasm"
    FINAL_WASM="$FINAL_WASM_DIR/${CANISTER_NAME}${SUFFIX}.wasm"
    DID_FILE="$FINAL_WASM_DIR/can${SUFFIX}.did"
    FINAL_GZIP="$FINAL_WASM_DIR/${CANISTER_NAME}_canister${SUFFIX}.wasm.gz"

    # Ensure destination directory exists
    mkdir -p "$FINAL_WASM_DIR"

    # 1. Compile
    cargo rustc --crate-type=cdylib --target wasm32-unknown-unknown "${FEATURE_ARGS[@]}" --target-dir "$BASE_CANISTER_PATH/$CANISTER_NAME/impl/target" --release --locked -p $CANISTER &&

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
    gzip -v -t "$FINAL_GZIP" ||
    FAILED+=("${CANISTER_NAME}${SUFFIX}")

    echo "Finished building canister: $CANISTER"
done

if [ ${#FAILED[@]} -ne 0 ]; then
    echo "Build FAILED for: ${FAILED[*]}" >&2
    exit 1
fi

echo "All canisters built successfully!"
