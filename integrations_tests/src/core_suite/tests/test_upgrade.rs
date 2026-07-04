use crate::client::core_nft::{
    finalize_upload, get_upload_status, icrc7_owner_of, icrc7_total_supply, init_upload,
    store_chunk,
};
use crate::client::pocket::execute_query;
use crate::core_suite::setup::old_test_setup;
use crate::core_suite::setup::setup::TestEnv;
use crate::core_suite::setup::setup_core::upgrade_core_canister;
use crate::utils::{create_default_metadata, mint_nft, tick_n_blocks};
use bity_ic_storage_canister_api::types::storage::UploadState;
use bity_ic_types::BuildVersion;
use candid::{Nat, Principal};
use core_nft_api::lifecycle::Args;
use core_nft_api::post_upgrade::UpgradeArgs;
use core_nft_common::types::management::{finalize_upload, get_upload_status, init_upload, store_chunk};
use ic_cdk::println;
use icrc_ledger_types::icrc1::account::Account;
use pocket_ic::PocketIc;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

fn upload_file_as(
    pic: &mut PocketIc,
    sender: Principal,
    collection_canister_id: Principal,
    file_path: &str,
    buffer: &[u8],
) {
    let mut hasher = Sha256::new();
    hasher.update(buffer);
    let file_hash = hasher.finalize();

    let init_upload_resp = init_upload(
        pic,
        sender,
        collection_canister_id,
        &(init_upload::Args {
            file_path: file_path.to_string(),
            file_hash: format!("{:x}", file_hash),
            file_size: buffer.len() as u64,
            chunk_size: None,
        }),
    );
    assert!(
        init_upload_resp.is_ok(),
        "init_upload failed: {init_upload_resp:?}"
    );

    let chunk_size = 1024 * 1024;
    for (chunk_index, chunk) in buffer.chunks(chunk_size).enumerate() {
        let store_chunk_resp = store_chunk(
            pic,
            sender,
            collection_canister_id,
            &(store_chunk::Args {
                file_path: file_path.to_string(),
                chunk_id: Nat::from(chunk_index as u64),
                chunk_data: chunk.to_vec(),
            }),
        );
        assert!(
            store_chunk_resp.is_ok(),
            "store_chunk failed: {store_chunk_resp:?}"
        );
    }

    let finalize_upload_resp = finalize_upload(
        pic,
        sender,
        collection_canister_id,
        &(finalize_upload::Args {
            file_path: file_path.to_string(),
        }),
    );
    assert!(
        finalize_upload_resp.is_ok(),
        "finalize_upload failed: {finalize_upload_resp:?}"
    );
}

fn storage_subcanisters(
    pic: &PocketIc,
    sender: Principal,
    collection_canister_id: Principal,
) -> Vec<Principal> {
    execute_query(
        pic,
        sender,
        collection_canister_id,
        "get_all_storage_subcanisters",
        &(),
    )
}

fn expected_storage_wasm_hash() -> Vec<u8> {
    let mut file = File::open(Path::new("../wasm/storage_canister.wasm.gz"))
        .expect("Failed to open storage canister wasm");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("Failed to read storage canister wasm");
    Sha256::digest(&buffer).to_vec()
}

// Upgrades a collection created by a previous-generation wasm (no content
// systems, no vetkd, no base_url) to the current build and verifies:
// tokens survive, legacy uploads stay visible and served, the storage
// sub-canister fleet is actually upgraded, and a repeated upgrade of the
// current build does not wipe any state.
#[test]
fn test_upgrade_storage_canister() {
    let mut test_env: TestEnv = old_test_setup();
    println!("test_env: {:?}", test_env);
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;

    // Pre-upgrade fleet state: one minted token and one uploaded file.
    let owner_account = Account {
        owner: nft_owner1,
        subaccount: None,
    };
    let token_id = mint_nft(
        pic,
        owner_account,
        controller,
        collection_canister_id,
        create_default_metadata(),
    )
    .expect("pre-upgrade mint failed");

    let file_path = Path::new("./src/core_suite/assets/test.png");
    let mut file = File::open(file_path).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    upload_file_as(pic, controller, collection_canister_id, "/test.png", &buffer);

    let supply_before = icrc7_total_supply(pic, controller, collection_canister_id, &());
    let owner_before = icrc7_owner_of(
        pic,
        controller,
        collection_canister_id,
        &vec![token_id.clone()],
    );
    let storage_before = storage_subcanisters(pic, controller, collection_canister_id);
    assert!(
        !storage_before.is_empty(),
        "the upload should have created a storage sub-canister"
    );

    // Upgrade to the current build.
    let storage_upgrade_args = Args::Upgrade(UpgradeArgs {
        version: BuildVersion::min(),
        commit_hash: "commit_hash 2".to_string(),
        vetkd_key_name: None,
        vetkd_context: None,
        base_url: Some("test".to_string()),
    });

    upgrade_core_canister(
        pic,
        collection_canister_id,
        storage_upgrade_args,
        controller,
    );

    // Let the zero-delay timer run the storage fleet upgrade
    // (stop / install / start per sub-canister, with retries).
    pic.advance_time(Duration::from_secs(1));
    tick_n_blocks(pic, 30);

    // Token state survived the migration.
    let supply_after = icrc7_total_supply(pic, controller, collection_canister_id, &());
    assert_eq!(supply_before, supply_after);
    let owner_after = icrc7_owner_of(
        pic,
        controller,
        collection_canister_id,
        &vec![token_id.clone()],
    );
    assert_eq!(owner_before, owner_after);

    // The legacy upload is visible through the new content system.
    let legacy_status = get_upload_status(
        pic,
        controller,
        collection_canister_id,
        &"/test.png".to_string(),
    );
    assert!(
        matches!(legacy_status, Ok(UploadState::Finalized)),
        "legacy upload must stay visible after the upgrade, got: {legacy_status:?}"
    );

    // The storage sub-canister fleet was upgraded to the embedded wasm.
    let storage_after = storage_subcanisters(pic, controller, collection_canister_id);
    assert_eq!(storage_before, storage_after);
    let expected_hash = expected_storage_wasm_hash();
    for storage_canister_id in &storage_after {
        let status = pic
            .canister_status(*storage_canister_id, Some(collection_canister_id))
            .expect("canister_status on storage canister failed");
        assert_eq!(
            status.module_hash,
            Some(expected_hash.clone()),
            "storage canister {storage_canister_id} was not upgraded"
        );
    }

    // A new upload reuses the upgraded storage canister instead of creating
    // a fresh one.
    upload_file_as(
        pic,
        controller,
        collection_canister_id,
        "/test2.png",
        &buffer,
    );
    tick_n_blocks(pic, 5);
    let storage_after_upload = storage_subcanisters(pic, controller, collection_canister_id);
    assert_eq!(storage_after, storage_after_upload);

    // A second upgrade of the current build must not wipe any state.
    let repeat_upgrade_args = Args::Upgrade(UpgradeArgs {
        version: BuildVersion::min(),
        commit_hash: "commit_hash 3".to_string(),
        vetkd_key_name: None,
        vetkd_context: None,
        base_url: None,
    });
    upgrade_core_canister(pic, collection_canister_id, repeat_upgrade_args, controller);
    pic.advance_time(Duration::from_secs(1));
    tick_n_blocks(pic, 30);

    let supply_repeat = icrc7_total_supply(pic, controller, collection_canister_id, &());
    assert_eq!(supply_before, supply_repeat);
    for path in ["/test.png", "/test2.png"] {
        let status = get_upload_status(pic, controller, collection_canister_id, &path.to_string());
        assert!(
            matches!(status, Ok(UploadState::Finalized)),
            "upload {path} must survive a repeated upgrade, got: {status:?}"
        );
    }
}
