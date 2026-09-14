#!/bin/bash

# Usage: ./scripts/build.sh [--inttest]
#
# Default (no flag): release build. The artifacts under src/<canister>/wasm/ are
# what we publish and what other repos embed, so they must never contain the
# test-only endpoints gated behind the `inttest` cargo feature
# (__get_public_entry_test, __get_private_entry_test, __get_premint_entry_test).
# A release build that extracts a candid still declaring them is a hard error,
# see the guard after the candid extraction below.
#
# --inttest: builds core_nft with `--features inttest` into the git-ignored
# src/core_nft/wasm/*_inttest.* paths and touches nothing else. The integration
# tests load that wasm; the release artifacts are left untouched. index_icrc7
# has no inttest-gated code, so it is never built this way and the tests use its
# release artifact.

BASE_CANISTER_PATH="./src"

# Storage sub-canister release to download. core_nft embeds the .gz with
# include_bytes!, so this decides which storage canister every collection
# deploys and upgrades its fleet to. It MUST match the version of the
# `bity-ic-storage-canister-*` crates in Cargo.toml: the embedded wasm and the
# candid the core compiles against have to be the same generation.
STORAGE_CANISTER_VERSION="0.7.0"
STORAGE_RELEASE_URL="https://github.com/BitySA/ic-storage-canister/releases/download/${STORAGE_CANISTER_VERSION}"

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

# Pinned, and failing loudly so an HTML error page is never written over the
# wasm. The CI image ships wget but not curl, so either tool is accepted.
fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -L --fail -o "$1" "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$1" "$2"
    else
        echo "neither curl nor wget is available to download $2" >&2
        return 1
    fi
}
mkdir -p "./wasm"
fetch "./wasm/storage_canister.wasm" "${STORAGE_RELEASE_URL}/storage_canister.wasm" || exit 1
fetch "./wasm/storage_canister.wasm.gz" "${STORAGE_RELEASE_URL}/storage_canister.wasm.gz" || exit 1

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

    # 2. Shrink & Optimize, reading cargo's output and writing the final
    # artifact. ic-wasm must never write back over $TARGET_WASM: with a warm
    # target dir cargo has nothing to rebuild, so the next run would shrink and
    # optimize an already optimized wasm and the bytes would drift from run to
    # run. Starting from cargo's untouched output keeps the artifact reproducible.
    ic-wasm "$TARGET_WASM" -o "$FINAL_WASM" shrink &&
    ic-wasm "$FINAL_WASM" -o "$FINAL_WASM" optimize --inline-functions-with-loops O3 &&

    # 3. Extract Candid from the optimized Wasm
    candid-extractor "$FINAL_WASM" > "$DID_FILE" &&

    # 4. Embed the Candid into the Wasm
    echo "Embedding candid metadata into $CANISTER..." &&
    ic-wasm "$FINAL_WASM" -o "$FINAL_WASM" metadata candid:service -f "$DID_FILE" -v public &&

    # 5. Gzip the Wasm (Now includes the metadata)
    gzip --no-name -9 -v -c "$FINAL_WASM" > "$FINAL_GZIP" &&
    gzip -v -t "$FINAL_GZIP" ||
    FAILED+=("${CANISTER_NAME}${SUFFIX}")

    # The shipped artifact must not expose the inttest-gated test queries. This
    # is a standalone block on purpose: `grep -q` exits 1 on the good case, so
    # chaining it with && above would mark every clean release build as failed.
    if [ "$INTTEST" = false ] && [ -f "$DID_FILE" ] && grep -q '__get_' "$DID_FILE"; then
        echo "release build exposes test endpoints: $DID_FILE" >&2
        exit 1
    fi

    echo "Finished building canister: $CANISTER"
done

if [ ${#FAILED[@]} -ne 0 ]; then
    echo "Build FAILED for: ${FAILED[*]}" >&2
    exit 1
fi

echo "All canisters built successfully!"
