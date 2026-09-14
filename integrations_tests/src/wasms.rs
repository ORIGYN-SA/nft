use bity_ic_types::CanisterWasm;
use lazy_static::lazy_static;
use std::fs::File;
use std::io::Read;

lazy_static! {
    // Wasms in wasms folder
    // pub static ref IC_ICRC1_LEDGER: CanisterWasm = get_canister_wasm("ic_icrc1_ledger");
    // pub static ref IC_ICRC2_LEDGER: CanisterWasm = get_canister_wasm_gz("icrc_ledger");
    // pub static ref SNS_GOVERNANCE: CanisterWasm = get_canister_wasm("sns_governance");
    // pub static ref SNS_ROOT: CanisterWasm = get_canister_wasm("sns_root");
    // pub static ref ICP_LEDGER: CanisterWasm = get_canister_wasm("ledger");
    // pub static ref REGISTRY_WASM: CanisterWasm = get_canister_wasm("registry");

    // Wasms in particular canister folder
    pub static ref CORE_WASM_OLD: CanisterWasm = get_core_wasm_old();

    // The tests call the endpoints gated behind the `inttest` cargo feature, so
    // they load the inttest build of core_nft, not the release artifact. The
    // released wasm must never expose those endpoints.
    // Build it with: ./scripts/build.sh --inttest
    pub static ref CORE_WASM: CanisterWasm = get_core_wasm_inttest();
    pub static ref INDEX_WASM: CanisterWasm = get_canister_wasm_from_bin("index_icrc7");
}

fn get_core_wasm_inttest() -> CanisterWasm {
    match read_file_from_relative_bin("../src/core_nft/wasm/core_nft_canister_inttest.wasm.gz") {
        Ok(wasm) => wasm,
        Err(err) => {
            println!(
                "Failed to read core_nft inttest wasm: {err}. \n\x1b[31mRun \"./scripts/build.sh --inttest\"\x1b[0m"
            );
            panic!()
        }
    }
}

/// The previous-generation core_nft wasm, checked in at `wasm/`. `test_upgrade`
/// and `test_media_serving` install it and then upload through the pre-0.7
/// candid, where `file_hash` is a bare `text`.
///
/// Whoever refreshes these bytes to a post-0.7 build MUST delete
/// `legacy_init_upload` in `client/core_nft.rs` in the same commit, and move
/// its callers onto `init_upload`. Candid decodes a `text` argument into an
/// `opt text` parameter, so those tests would keep passing against a 0.7 wasm
/// while proving nothing about the legacy shape they exist to cover.
fn get_core_wasm_old() -> CanisterWasm {
    match read_file_from_relative_bin(&format!("../wasm/core_nft_canister.wasm.gz")) {
        Ok(wasm) => wasm,
        Err(err) => {
            println!("Failed to read core_nft_api_old wasm: {err}");
            panic!()
        }
    }
}

fn get_canister_wasm_from_bin(canister_name: &str) -> CanisterWasm {
    match read_file_from_relative_bin(&format!(
        "../src/{canister_name}/wasm/{canister_name}_canister.wasm.gz"
    )) {
        Ok(wasm) => wasm,
        Err(err) => {
            println!(
                "Failed to read {canister_name} wasm: {err}. \n\x1b[31mRun \"./scripts/build_canister.sh {canister_name}\"\x1b[0m"
            );
            panic!()
        }
    }
}

fn read_file_from_relative_bin(file_path: &str) -> Result<Vec<u8>, std::io::Error> {
    // Open the wasm file
    let mut file = File::open(file_path)?;

    // Read the contents of the file into a vector
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}
