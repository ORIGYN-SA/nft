use crate::client::pocket::execute_query;
use crate::core_suite::setup::setup::TestEnv;
use crate::core_suite::setup::setup_core::upgrade_core_canister;
use crate::core_suite::setup::{
    default_test_setup, old_test_setup, test_setup_with_storage_cycles,
};
use crate::utils::{tick_n_blocks, upload_file};
use bity_ic_types::BuildVersion;
use candid::Principal;
use core_nft_api::lifecycle::Args;
use core_nft_api::post_upgrade::UpgradeArgs;
use core_nft_common::types::sub_canister::{StorageCyclesConfig, INITIAL_CYCLES_BALANCE_TEST_MODE};
use pocket_ic::PocketIc;
use std::time::Duration;

const FILE_PATH: &str = "./src/core_suite/assets/test.png";

// Above the 0.5T test-mode default, so a storage canister holding more than the
// default can only have been funded from the configured value.
const CONFIGURED_INITIAL_CYCLES: u128 = 700_000_000_000;

// What the previous-generation test wasm (`wasm/core_nft_canister.wasm.gz`) funds
// a test-mode storage canister with, as observed on a spawn.
const OLD_WASM_INITIAL_CYCLES_TEST_MODE: u128 = 5_000_000_000_000;

fn upgrade_with(
    pic: &mut PocketIc,
    controller: Principal,
    collection_canister_id: Principal,
    storage_cycles: Option<StorageCyclesConfig>,
) {
    // install_code is rate limited per canister, and the debit only drains as
    // rounds execute.
    tick_n_blocks(pic, 100);
    upgrade_core_canister(
        pic,
        collection_canister_id,
        Args::Upgrade(UpgradeArgs {
            version: BuildVersion::min(),
            commit_hash: "commit_hash 2".to_string(),
            vetkd_key_name: None,
            vetkd_context: None,
            base_url: None,
            storage_cycles,
        }),
        controller,
    );
    pic.advance_time(Duration::from_secs(1));
    tick_n_blocks(pic, 10);
}

/// Uploads one file, which makes the collection spawn its first storage
/// canister, and returns that canister's cycle balance.
fn spawn_storage_canister(
    pic: &mut PocketIc,
    controller: Principal,
    collection: Principal,
) -> u128 {
    upload_file(
        pic,
        controller,
        collection,
        FILE_PATH,
        "/storage_cycles.png",
    )
    .expect("upload failed");

    let storage_canisters: Vec<Principal> = execute_query(
        pic,
        controller,
        collection,
        "get_all_storage_subcanisters",
        &(),
    );
    assert_eq!(
        storage_canisters.len(),
        1,
        "expected exactly one storage canister"
    );

    pic.cycle_balance(storage_canisters[0])
}

#[test]
fn test_storage_cycles_from_init_args() {
    let mut test_env: TestEnv = test_setup_with_storage_cycles(Some(StorageCyclesConfig {
        initial_cycles: Some(CONFIGURED_INITIAL_CYCLES),
        ..Default::default()
    }));
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    let balance = spawn_storage_canister(pic, controller, collection_canister_id);

    assert!(
        balance > INITIAL_CYCLES_BALANCE_TEST_MODE && balance <= CONFIGURED_INITIAL_CYCLES,
        "storage canister should hold the configured {CONFIGURED_INITIAL_CYCLES}, got {balance}"
    );
}

#[test]
fn test_storage_cycles_from_upgrade_args_are_kept() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    upgrade_with(
        pic,
        controller,
        collection_canister_id,
        Some(StorageCyclesConfig {
            initial_cycles: Some(CONFIGURED_INITIAL_CYCLES),
            ..Default::default()
        }),
    );
    // An upgrade without settings must keep the stored ones.
    upgrade_with(pic, controller, collection_canister_id, None);

    let balance = spawn_storage_canister(pic, controller, collection_canister_id);

    assert!(
        balance > INITIAL_CYCLES_BALANCE_TEST_MODE && balance <= CONFIGURED_INITIAL_CYCLES,
        "storage canister should hold the configured {CONFIGURED_INITIAL_CYCLES}, got {balance}"
    );
}

#[test]
fn test_storage_cycles_untouched_without_args() {
    let mut test_env: TestEnv = old_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        ..
    } = test_env;

    // A collection that never received settings keeps the spawn numbers it was
    // created with instead of picking up the current defaults.
    upgrade_with(pic, controller, collection_canister_id, None);

    let balance = spawn_storage_canister(pic, controller, collection_canister_id);

    assert!(
        balance > INITIAL_CYCLES_BALANCE_TEST_MODE && balance <= OLD_WASM_INITIAL_CYCLES_TEST_MODE,
        "storage canister should hold the old wasm's {OLD_WASM_INITIAL_CYCLES_TEST_MODE}, got {balance}"
    );
}
