//! Permanent collections: mint blocks carry the first version, burns are
//! refused, edits always log the full previous and new metadata, and the
//! setting survives an upgrade. A collection installed without the setting
//! keeps the old behaviour, including mint blocks without `meta`.

use crate::client::core_nft::{
    burn_nft, get_collection_settings, icrc3_get_blocks, icrc7_token_metadata, mint,
    update_nft_metadata_batch,
};
use crate::core_suite::setup::setup::TestEnv;
use crate::core_suite::setup::setup_core::upgrade_core_canister;
use crate::core_suite::setup::{claimlink_init_args_test_setup, permanent_test_setup};
use crate::utils::tick_n_blocks;
use bity_ic_types::BuildVersion;
use candid::{Nat, Principal};
use core_nft_api::lifecycle::Args;
use core_nft_api::post_upgrade::UpgradeArgs;
use core_nft_common::types::management::burn_nft::BurnNftError;
use core_nft_common::types::management::get_collection_settings::CollectionSettings;
use core_nft_common::types::management::update_nft_metadata::UpdateNftMetadataError;
use core_nft_common::types::management::update_nft_metadata_batch::{Item, LogMode};
use core_nft_common::types::management::{mint, update_nft_metadata_batch};
use icrc_ledger_types::icrc::generic_value::ICRC3Value;
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc3::blocks::GetBlocksRequest;
use pocket_ic::PocketIc;
use std::collections::BTreeMap;
use std::time::Duration;

fn json_data(text: &str) -> Vec<(String, ICRC3Value)> {
    vec![("JSON_DATA".to_string(), ICRC3Value::Text(text.to_string()))]
}

/// Mints one token owned by `owner` and moves the clock past the log's window.
fn mint_one(
    pic: &mut PocketIc,
    controller: Principal,
    collection: Principal,
    owner: Principal,
    json: &str,
) -> Nat {
    let token_id = mint(
        pic,
        controller,
        collection,
        &mint::Args {
            mint_requests: vec![mint::MintRequest {
                token_owner: Account {
                    owner,
                    subaccount: None,
                },
                memo: None,
                metadata: json_data(json),
                private_content: None,
                public_content: None,
            }],
        },
    )
    .expect("mint");
    pic.advance_time(Duration::from_secs(1));
    pic.tick();
    token_id
}

fn log_length(pic: &PocketIc, controller: Principal, collection: Principal) -> Nat {
    icrc3_get_blocks(
        pic,
        controller,
        collection,
        &vec![GetBlocksRequest {
            start: Nat::from(0u64),
            length: Nat::from(0u64),
        }],
    )
    .log_length
}

/// The `tx` map of the block at `index`.
fn block_tx(
    pic: &PocketIc,
    controller: Principal,
    collection: Principal,
    index: &Nat,
) -> BTreeMap<String, ICRC3Value> {
    let result = icrc3_get_blocks(
        pic,
        controller,
        collection,
        &vec![GetBlocksRequest {
            start: index.clone(),
            length: Nat::from(1u64),
        }],
    );
    assert_eq!(result.blocks.len(), 1, "no local block at index {}", index);
    let block = match &result.blocks[0].block {
        ICRC3Value::Map(block) => block.clone(),
        other => panic!("block {} is not a map: {:?}", index, other),
    };
    match block.get("tx") {
        Some(ICRC3Value::Map(tx)) => tx.clone(),
        other => panic!("block {} has no tx map: {:?}", index, other),
    }
}

fn token_metadata_map(
    pic: &PocketIc,
    controller: Principal,
    collection: Principal,
    token_id: &Nat,
) -> ICRC3Value {
    let metadata = icrc7_token_metadata(pic, controller, collection, &vec![token_id.clone()])
        .remove(0)
        .expect("the token has metadata");
    ICRC3Value::Map(metadata.into_iter().collect())
}

fn upgrade(pic: &mut PocketIc, controller: Principal, collection: Principal) {
    // install_code is rate limited per canister, and the debit only drains as
    // rounds execute.
    tick_n_blocks(pic, 100);
    upgrade_core_canister(
        pic,
        collection,
        Args::Upgrade(UpgradeArgs {
            version: BuildVersion::min(),
            commit_hash: "commit_hash 2".to_string(),
            vetkd_key_name: None,
            vetkd_context: None,
            base_url: None,
            storage_cycles: None,
        }),
        controller,
    );
    pic.advance_time(Duration::from_secs(1));
    tick_n_blocks(pic, 10);
}

#[test]
fn test_normal_mint_block_still_has_no_meta() {
    let mut test_env: TestEnv = claimlink_init_args_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;

    mint_one(pic, controller, collection_canister_id, nft_owner1, r#"{"a":1}"#);

    let tx = block_tx(pic, controller, collection_canister_id, &Nat::from(0u64));
    assert_eq!(tx.get("op"), Some(&ICRC3Value::Text("7mint".to_string())));
    assert_eq!(tx.get("meta"), None);
}

#[test]
fn test_permanent_burn_is_refused() {
    let mut test_env: TestEnv = permanent_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_id = mint_one(pic, controller, collection_canister_id, nft_owner1, r#"{"a":1}"#);
    let before = log_length(pic, controller, collection_canister_id);

    let result = burn_nft(pic, nft_owner1, collection_canister_id, &token_id);

    assert!(
        matches!(result, Err(BurnNftError::CollectionIsPermanent)),
        "expected CollectionIsPermanent, got {:?}",
        result
    );
    assert_eq!(log_length(pic, controller, collection_canister_id), before);
    assert!(icrc7_token_metadata(
        pic,
        controller,
        collection_canister_id,
        &vec![token_id.clone()]
    )[0]
        .is_some());
}

#[test]
fn test_normal_burn_still_works() {
    let mut test_env: TestEnv = claimlink_init_args_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_id = mint_one(pic, controller, collection_canister_id, nft_owner1, r#"{"a":1}"#);

    let result = burn_nft(pic, nft_owner1, collection_canister_id, &token_id);

    assert!(result.is_ok(), "burn failed: {:?}", result);
}

#[test]
fn test_permanent_hash_only_edit_is_refused_and_full_edit_works() {
    let mut test_env: TestEnv = permanent_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_id = mint_one(pic, controller, collection_canister_id, nft_owner1, r#"{"a":1}"#);
    let original = token_metadata_map(pic, controller, collection_canister_id, &token_id);
    let before = log_length(pic, controller, collection_canister_id);
    let edit = |log: LogMode| update_nft_metadata_batch::Args {
        items: vec![Item {
            token_id: token_id.clone(),
            metadata: json_data(r#"{"a":2}"#),
        }],
        memo: None,
        log,
    };

    let refused = update_nft_metadata_batch(
        pic,
        controller,
        collection_canister_id,
        &edit(LogMode::HashOnly),
    );
    assert!(
        matches!(refused, Err(UpdateNftMetadataError::FullLogRequired)),
        "expected FullLogRequired, got {:?}",
        refused
    );
    assert_eq!(log_length(pic, controller, collection_canister_id), before);
    assert_eq!(
        token_metadata_map(pic, controller, collection_canister_id, &token_id),
        original
    );

    let block_indexes = update_nft_metadata_batch(
        pic,
        controller,
        collection_canister_id,
        &edit(LogMode::Full),
    )
    .expect("a full edit is accepted");
    let tx = block_tx(pic, controller, collection_canister_id, &block_indexes[0]);
    let meta = match tx.get("meta") {
        Some(ICRC3Value::Map(meta)) => meta.clone(),
        other => panic!("the edit block has no meta map: {:?}", other),
    };
    assert_eq!(meta.get("icrc7:previous_metadata"), Some(&original));
}

#[test]
fn test_collection_settings_report_the_install_choice() {
    let permanent: TestEnv = permanent_test_setup();
    assert_eq!(
        get_collection_settings(
            &permanent.pic,
            permanent.controller,
            permanent.collection_canister_id,
            &()
        ),
        CollectionSettings { permanent: true }
    );

    let normal: TestEnv = claimlink_init_args_test_setup();
    assert_eq!(
        get_collection_settings(
            &normal.pic,
            normal.controller,
            normal.collection_canister_id,
            &()
        ),
        CollectionSettings { permanent: false }
    );
}

#[test]
fn test_permanent_survives_an_upgrade() {
    let mut test_env: TestEnv = permanent_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_id = mint_one(pic, controller, collection_canister_id, nft_owner1, r#"{"a":1}"#);

    upgrade(pic, controller, collection_canister_id);

    assert_eq!(
        get_collection_settings(pic, controller, collection_canister_id, &()),
        CollectionSettings { permanent: true }
    );
    let result = burn_nft(pic, nft_owner1, collection_canister_id, &token_id);
    assert!(
        matches!(result, Err(BurnNftError::CollectionIsPermanent)),
        "expected CollectionIsPermanent after the upgrade, got {:?}",
        result
    );
}

fn mint_meta(tx: &BTreeMap<String, ICRC3Value>) -> ICRC3Value {
    match tx.get("meta") {
        Some(ICRC3Value::Map(meta)) => meta
            .get("icrc7:token_metadata")
            .cloned()
            .expect("the mint meta holds icrc7:token_metadata"),
        other => panic!("the mint block has no meta map: {:?}", other),
    }
}

#[test]
fn test_permanent_mint_block_carries_the_first_version() {
    let mut test_env: TestEnv = permanent_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_id = mint_one(pic, controller, collection_canister_id, nft_owner1, r#"{"a":1}"#);
    let first_version = token_metadata_map(pic, controller, collection_canister_id, &token_id);

    let mint_tx = block_tx(pic, controller, collection_canister_id, &Nat::from(0u64));
    assert_eq!(mint_tx.get("op"), Some(&ICRC3Value::Text("7mint".to_string())));
    assert_eq!(mint_meta(&mint_tx), first_version);

    let block_indexes = update_nft_metadata_batch(
        pic,
        controller,
        collection_canister_id,
        &update_nft_metadata_batch::Args {
            items: vec![Item {
                token_id: token_id.clone(),
                metadata: json_data(r#"{"a":2}"#),
            }],
            memo: None,
            log: LogMode::Full,
        },
    )
    .expect("a full edit is accepted");
    let edit_tx = block_tx(pic, controller, collection_canister_id, &block_indexes[0]);
    match edit_tx.get("meta") {
        Some(ICRC3Value::Map(meta)) => {
            assert_eq!(meta.get("icrc7:previous_metadata"), Some(&first_version))
        }
        other => panic!("the edit block has no meta map: {:?}", other),
    }
}

/// One mint call close to the 2 MiB message limit: every block carries its
/// certificate, and the chain log's duplicate check hashes each one against
/// the others in the same call.
#[test]
fn test_permanent_mint_batch_at_the_message_limit() {
    const ITEMS: usize = 40;
    const CERTIFICATE_BYTES: usize = 45_000;
    let mut test_env: TestEnv = permanent_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let mint_requests: Vec<mint::MintRequest> = (0..ITEMS)
        .map(|n| mint::MintRequest {
            token_owner: Account {
                owner: nft_owner1,
                subaccount: None,
            },
            memo: None,
            metadata: json_data(&format!(
                r#"{{"n":{n},"body":"{}"}}"#,
                "x".repeat(CERTIFICATE_BYTES)
            )),
            private_content: None,
            public_content: None,
        })
        .collect();
    let cycles_before = pic.cycle_balance(collection_canister_id);

    mint(
        pic,
        controller,
        collection_canister_id,
        &mint::Args { mint_requests },
    )
    .expect("the batch mints");

    println!(
        "cycles for {ITEMS} permanent mints of {CERTIFICATE_BYTES} bytes: {}",
        cycles_before - pic.cycle_balance(collection_canister_id)
    );
    pic.advance_time(Duration::from_secs(1));
    pic.tick();
    assert_eq!(log_length(pic, controller, collection_canister_id), Nat::from(ITEMS as u64));
    for index in [0u64, ITEMS as u64 - 1] {
        let tx = block_tx(pic, controller, collection_canister_id, &Nat::from(index));
        match mint_meta(&tx) {
            ICRC3Value::Map(metadata) => assert!(matches!(
                metadata.get("JSON_DATA"),
                Some(ICRC3Value::Text(text)) if text.len() > CERTIFICATE_BYTES
            )),
            other => panic!("the token metadata is not a map: {:?}", other),
        }
    }
}
