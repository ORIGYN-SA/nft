//! `update_nft_metadata` and `update_nft_metadata_batch`: the memo lands in
//! the `7update_token` block, the call answers with that block's index, and a
//! block the ICRC-3 log refuses leaves no metadata change behind.

use crate::client::core_nft::{
    icrc3_get_blocks, icrc7_token_metadata, mint, update_nft_metadata, update_nft_metadata_batch,
};
use crate::core_suite::setup::setup::TestEnv;
use crate::core_suite::setup::test_setup_no_limit;
use candid::{Nat, Principal};
use core_nft_common::types::icrc7::DEFAULT_MAX_MEMO_SIZE;
use core_nft_common::types::management::update_nft_metadata_batch::{Item, LogMode};
use core_nft_common::types::management::{mint, update_nft_metadata, update_nft_metadata_batch};
use icrc_ledger_types::icrc::generic_value::ICRC3Value;
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc3::blocks::GetBlocksRequest;
use pocket_ic::{PocketIc, RejectResponse};
use serde_bytes::ByteBuf;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::time::Duration;

/// Tokens per `mint` call. Under a zero `tx_window` one message can add at most
/// 50 blocks, so minting every token in one call would trap.
const MINT_CHUNK: usize = 20;

fn json_data(text: &str) -> Vec<(String, ICRC3Value)> {
    vec![("JSON_DATA".to_string(), ICRC3Value::Text(text.to_string()))]
}

fn map_of(entries: &[(String, ICRC3Value)]) -> ICRC3Value {
    ICRC3Value::Map(entries.iter().cloned().collect())
}

/// Mints `count` tokens owned by `owner`, `MINT_CHUNK` per call, the token at
/// position `p` carrying `JSON_DATA` = `{"n":p}`. The clock moves one second
/// after every call, past the 100 ms a block stays in the log's window, so none
/// of these blocks count against a later message.
fn mint_tokens(
    pic: &mut PocketIc,
    controller: Principal,
    collection: Principal,
    owner: Principal,
    count: usize,
) -> Vec<Nat> {
    let mut token_ids = Vec::with_capacity(count);
    while token_ids.len() < count {
        let start = token_ids.len();
        let chunk = (count - start).min(MINT_CHUNK);
        let first = mint(
            pic,
            controller,
            collection,
            &mint::Args {
                mint_requests: (start..start + chunk)
                    .map(|position| mint::MintRequest {
                        token_owner: Account {
                            owner,
                            subaccount: None,
                        },
                        memo: None,
                        metadata: json_data(&format!("{{\"n\":{}}}", position)),
                        private_content: None,
                        public_content: None,
                    })
                    .collect(),
            },
        )
        .expect("mint");
        for offset in 0..chunk {
            token_ids.push(first.clone() + Nat::from(offset as u64));
        }
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
    }
    token_ids
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

/// The block at `index`. A collection archives only past 100,000 local blocks,
/// so every block of these tests is served by the collection itself.
fn block_at(
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
    assert_eq!(&result.blocks[0].id, index);
    match &result.blocks[0].block {
        ICRC3Value::Map(block) => block.clone(),
        other => panic!("block {} is not a map: {:?}", index, other),
    }
}

fn field_map(map: &BTreeMap<String, ICRC3Value>, key: &str) -> BTreeMap<String, ICRC3Value> {
    match map.get(key) {
        Some(ICRC3Value::Map(inner)) => inner.clone(),
        other => panic!("`{}` is not a map: {:?}", key, other),
    }
}

fn token_metadata(
    pic: &PocketIc,
    controller: Principal,
    collection: Principal,
    token_id: &Nat,
) -> Option<Vec<(String, ICRC3Value)>> {
    icrc7_token_metadata(pic, controller, collection, &vec![token_id.clone()]).remove(0)
}

/// `update_nft_metadata` without the client's panic on a reject, for calls
/// that are expected to trap.
fn update_nft_metadata_raw(
    pic: &PocketIc,
    sender: Principal,
    collection: Principal,
    args: &update_nft_metadata::Args,
) -> Result<update_nft_metadata::Response, RejectResponse> {
    pic.update_call(
        collection,
        sender,
        "update_nft_metadata",
        candid::encode_one(args).unwrap(),
    )
    .map(|bytes| candid::decode_one(&bytes).unwrap())
}

#[test]
fn test_update_nft_metadata_memo_lands_in_block() {
    let mut test_env: TestEnv = test_setup_no_limit();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_ids = mint_tokens(pic, controller, collection_canister_id, nft_owner1, 1);
    let memo = ByteBuf::from(br#"{"k":"edit","by":"aaaaa-aa","mint_request":88}"#.to_vec());

    let block_index = update_nft_metadata(
        pic,
        controller,
        collection_canister_id,
        &update_nft_metadata::Args {
            token_id: token_ids[0].clone(),
            metadata: json_data(r#"{"n":"edited"}"#),
            memo: Some(memo.clone()),
        },
    )
    .expect("update_nft_metadata");

    let tx = field_map(
        &block_at(pic, controller, collection_canister_id, &block_index),
        "tx",
    );
    assert_eq!(
        tx.get("op"),
        Some(&ICRC3Value::Text("7update_token".to_string()))
    );
    assert_eq!(tx.get("memo"), Some(&ICRC3Value::Blob(memo)));
}

#[test]
fn test_update_nft_metadata_oversized_memo_refused() {
    let mut test_env: TestEnv = test_setup_no_limit();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_ids = mint_tokens(pic, controller, collection_canister_id, nft_owner1, 1);
    let before = log_length(pic, controller, collection_canister_id);
    // `max_memo_size` is unset, so the limit is DEFAULT_MAX_MEMO_SIZE.
    let memo = ByteBuf::from(vec![0u8; DEFAULT_MAX_MEMO_SIZE as usize + 1]);

    let result = update_nft_metadata(
        pic,
        controller,
        collection_canister_id,
        &update_nft_metadata::Args {
            token_id: token_ids[0].clone(),
            metadata: json_data(r#"{"n":"edited"}"#),
            memo: Some(memo),
        },
    );

    assert!(
        matches!(
            result,
            Err(update_nft_metadata::UpdateNftMetadataError::InvalidMemo(_))
        ),
        "expected InvalidMemo, got {:?}",
        result
    );
    assert_eq!(log_length(pic, controller, collection_canister_id), before);
    assert_eq!(
        token_metadata(pic, controller, collection_canister_id, &token_ids[0]),
        Some(json_data(r#"{"n":0}"#))
    );
}

#[test]
fn test_update_nft_metadata_returns_block_index() {
    let mut test_env: TestEnv = test_setup_no_limit();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_ids = mint_tokens(pic, controller, collection_canister_id, nft_owner1, 1);
    let before = log_length(pic, controller, collection_canister_id);

    let first = update_nft_metadata(
        pic,
        controller,
        collection_canister_id,
        &update_nft_metadata::Args {
            token_id: token_ids[0].clone(),
            metadata: json_data(r#"{"n":"first edit"}"#),
            memo: None,
        },
    )
    .expect("first update");
    let second = update_nft_metadata(
        pic,
        controller,
        collection_canister_id,
        &update_nft_metadata::Args {
            token_id: token_ids[0].clone(),
            metadata: json_data(r#"{"n":"second edit"}"#),
            memo: None,
        },
    )
    .expect("second update");

    assert_eq!(first, before, "the new block sits at the old log length");
    assert_eq!(second, first.clone() + Nat::from(1u64));
    assert_eq!(
        log_length(pic, controller, collection_canister_id),
        before + Nat::from(2u64)
    );

    let block = block_at(pic, controller, collection_canister_id, &first);
    assert_eq!(
        block.get("btype"),
        Some(&ICRC3Value::Text("7update_token".to_string()))
    );
    let tx = field_map(&block, "tx");
    assert_eq!(tx.get("tid"), Some(&ICRC3Value::Nat(token_ids[0].clone())));
    assert_eq!(tx.get("memo"), None);
    let meta = field_map(&tx, "meta");
    assert_eq!(
        meta.get("icrc7:previous_metadata"),
        Some(&map_of(&json_data(r#"{"n":0}"#)))
    );
    assert_eq!(
        meta.get("icrc7:new_metadata"),
        Some(&map_of(&json_data(r#"{"n":"first edit"}"#)))
    );
}

#[test]
fn test_update_nft_metadata_log_failure_keeps_metadata() {
    let mut test_env: TestEnv = test_setup_no_limit();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_ids = mint_tokens(pic, controller, collection_canister_id, nft_owner1, 1);

    // The clock stays still from here on, so every block below stays inside
    // the log's window until the throttle refuses one.
    let mut current = json_data(r#"{"n":0}"#);
    let mut refusal = None;
    for attempt in 0..60u32 {
        let before = log_length(pic, controller, collection_canister_id);
        let metadata = json_data(&format!("{{\"n\":\"edit {}\"}}", attempt));
        let args = update_nft_metadata::Args {
            token_id: token_ids[0].clone(),
            metadata: metadata.clone(),
            memo: None,
        };
        match update_nft_metadata_raw(pic, controller, collection_canister_id, &args) {
            Ok(response) => {
                response.expect("an accepted call answers with a block index");
                current = metadata;
            }
            Err(reject) => {
                refusal = Some((attempt, before, reject));
                break;
            }
        }
    }

    let (attempt, before, reject) = refusal.expect("the log refused no block within 60 calls");
    println!(
        "update_nft_metadata: the log refused call {} after {} accepted",
        attempt + 1,
        attempt
    );
    assert!(
        reject.reject_message.contains("Transaction throttled"),
        "unexpected reject: {}",
        reject.reject_message
    );
    assert_eq!(
        log_length(pic, controller, collection_canister_id),
        before,
        "the refused call left no block"
    );
    assert_eq!(
        token_metadata(pic, controller, collection_canister_id, &token_ids[0]),
        Some(current),
        "the refused call left no metadata change"
    );
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&Sha256::digest(bytes));
    out
}

/// The ICRC-3 representation-independent hash, written here from the standard
/// rather than taken from a library, so the test does not check the canister
/// against the canister's own code.
fn independent_hash(value: &ICRC3Value) -> [u8; 32] {
    match value {
        ICRC3Value::Blob(bytes) => sha256(bytes),
        ICRC3Value::Text(text) => sha256(text.as_bytes()),
        ICRC3Value::Nat(nat) => {
            let mut rest = u128::try_from(nat.0.clone()).expect("test values fit in u128");
            let mut leb128 = Vec::new();
            loop {
                let byte = (rest & 0x7f) as u8;
                rest >>= 7;
                if rest == 0 {
                    leb128.push(byte);
                    break;
                }
                leb128.push(byte | 0x80);
            }
            sha256(&leb128)
        }
        ICRC3Value::Int(_) => panic!("these tests never store an Int"),
        ICRC3Value::Array(values) => {
            let mut concatenated = Vec::new();
            for value in values {
                concatenated.extend_from_slice(&independent_hash(value));
            }
            sha256(&concatenated)
        }
        ICRC3Value::Map(map) => {
            let mut pairs: Vec<([u8; 32], [u8; 32])> = map
                .iter()
                .map(|(key, value)| (sha256(key.as_bytes()), independent_hash(value)))
                .collect();
            pairs.sort();
            let mut concatenated = Vec::new();
            for (key_hash, value_hash) in pairs {
                concatenated.extend_from_slice(&key_hash);
                concatenated.extend_from_slice(&value_hash);
            }
            sha256(&concatenated)
        }
    }
}

const VISIBILITY_MEMO: &[u8] = br#"{"k":"visibility","to":"private"}"#;

#[test]
fn test_update_nft_metadata_batch_writes_one_block_per_item_in_order() {
    let mut test_env: TestEnv = test_setup_no_limit();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_ids = mint_tokens(pic, controller, collection_canister_id, nft_owner1, 3);
    let memo = ByteBuf::from(VISIBILITY_MEMO.to_vec());
    let positions = [2usize, 0, 1];
    let items: Vec<Item> = positions
        .iter()
        .map(|&position| Item {
            token_id: token_ids[position].clone(),
            metadata: json_data(&format!("{{\"batch\":{}}}", position)),
        })
        .collect();
    let before = log_length(pic, controller, collection_canister_id);

    let block_indexes = update_nft_metadata_batch(
        pic,
        controller,
        collection_canister_id,
        &update_nft_metadata_batch::Args {
            items: items.clone(),
            memo: Some(memo.clone()),
            log: LogMode::Full,
        },
    )
    .expect("update_nft_metadata_batch");

    let expected: Vec<Nat> = (0..3u64).map(|k| before.clone() + Nat::from(k)).collect();
    assert_eq!(block_indexes, expected, "one block per item, in item order");
    assert_eq!(
        log_length(pic, controller, collection_canister_id),
        before + Nat::from(3u64)
    );

    for ((block_index, item), position) in block_indexes.iter().zip(&items).zip(positions) {
        let tx = field_map(
            &block_at(pic, controller, collection_canister_id, block_index),
            "tx",
        );
        assert_eq!(
            tx.get("op"),
            Some(&ICRC3Value::Text("7update_token".to_string()))
        );
        assert_eq!(tx.get("tid"), Some(&ICRC3Value::Nat(item.token_id.clone())));
        assert_eq!(tx.get("memo"), Some(&ICRC3Value::Blob(memo.clone())));
        let meta = field_map(&tx, "meta");
        assert_eq!(
            meta.get("icrc7:previous_metadata"),
            Some(&map_of(&json_data(&format!("{{\"n\":{}}}", position))))
        );
        assert_eq!(
            meta.get("icrc7:new_metadata"),
            Some(&map_of(&item.metadata))
        );
        assert_eq!(
            token_metadata(pic, controller, collection_canister_id, &item.token_id),
            Some(item.metadata.clone())
        );
    }
}

#[test]
fn test_update_nft_metadata_batch_hash_only_matches_independent_hash() {
    let mut test_env: TestEnv = test_setup_no_limit();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_ids = mint_tokens(pic, controller, collection_canister_id, nft_owner1, 2);
    let previous: Vec<Vec<(String, ICRC3Value)>> = token_ids
        .iter()
        .map(|token_id| {
            token_metadata(pic, controller, collection_canister_id, token_id)
                .expect("a minted token has metadata")
        })
        .collect();
    let items: Vec<Item> = token_ids
        .iter()
        .map(|token_id| Item {
            token_id: token_id.clone(),
            metadata: vec![
                (
                    "JSON_DATA".to_string(),
                    ICRC3Value::Text(
                        r#"{"template":{"id":7,"version":3},"private":{"v":1,"salt":"c2FsdA==","blob":"YmxvYg=="}}"#
                            .to_string(),
                    ),
                ),
                // A Nat above 127 takes two LEB128 bytes.
                ("revision".to_string(), ICRC3Value::Nat(Nat::from(300u64))),
            ],
        })
        .collect();

    let block_indexes = update_nft_metadata_batch(
        pic,
        controller,
        collection_canister_id,
        &update_nft_metadata_batch::Args {
            items: items.clone(),
            memo: Some(ByteBuf::from(VISIBILITY_MEMO.to_vec())),
            log: LogMode::HashOnly,
        },
    )
    .expect("update_nft_metadata_batch");
    assert_eq!(block_indexes.len(), items.len());

    for ((block_index, item), previous) in block_indexes.iter().zip(&items).zip(&previous) {
        let meta = field_map(
            &field_map(
                &block_at(pic, controller, collection_canister_id, block_index),
                "tx",
            ),
            "meta",
        );
        let keys: Vec<String> = meta.keys().cloned().collect();
        assert_eq!(
            keys,
            vec![
                "icrc7:new_metadata_hash".to_string(),
                "icrc7:previous_metadata_hash".to_string()
            ],
            "HashOnly logs the two hashes and no map"
        );
        let new = token_metadata(pic, controller, collection_canister_id, &item.token_id)
            .expect("the token keeps its metadata");
        assert_eq!(map_of(&new), map_of(&item.metadata));
        assert_eq!(
            meta.get("icrc7:previous_metadata_hash"),
            Some(&ICRC3Value::Blob(ByteBuf::from(
                independent_hash(&map_of(previous)).to_vec()
            )))
        );
        assert_eq!(
            meta.get("icrc7:new_metadata_hash"),
            Some(&ICRC3Value::Blob(ByteBuf::from(
                independent_hash(&map_of(&new)).to_vec()
            )))
        );
    }
}

#[test]
fn test_update_nft_metadata_batch_missing_token_writes_nothing() {
    let mut test_env: TestEnv = test_setup_no_limit();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_ids = mint_tokens(pic, controller, collection_canister_id, nft_owner1, 1);
    let before = log_length(pic, controller, collection_canister_id);

    let result = update_nft_metadata_batch(
        pic,
        controller,
        collection_canister_id,
        &update_nft_metadata_batch::Args {
            items: vec![
                Item {
                    token_id: token_ids[0].clone(),
                    metadata: json_data(r#"{"n":"changed"}"#),
                },
                Item {
                    token_id: Nat::from(999_999u64),
                    metadata: json_data("{}"),
                },
            ],
            memo: None,
            log: LogMode::Full,
        },
    );

    assert!(
        matches!(
            result,
            Err(update_nft_metadata::UpdateNftMetadataError::TokenDoesNotExist)
        ),
        "expected TokenDoesNotExist, got {:?}",
        result
    );
    assert_eq!(log_length(pic, controller, collection_canister_id), before);
    assert_eq!(
        token_metadata(pic, controller, collection_canister_id, &token_ids[0]),
        Some(json_data(r#"{"n":0}"#)),
        "the existing token listed first was not written"
    );
}

#[test]
fn test_update_nft_metadata_batch_oversized_memo_refused() {
    let mut test_env: TestEnv = test_setup_no_limit();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_ids = mint_tokens(pic, controller, collection_canister_id, nft_owner1, 1);
    let before = log_length(pic, controller, collection_canister_id);

    let result = update_nft_metadata_batch(
        pic,
        controller,
        collection_canister_id,
        &update_nft_metadata_batch::Args {
            items: vec![Item {
                token_id: token_ids[0].clone(),
                metadata: json_data(r#"{"n":"changed"}"#),
            }],
            memo: Some(ByteBuf::from(vec![0u8; DEFAULT_MAX_MEMO_SIZE as usize + 1])),
            log: LogMode::HashOnly,
        },
    );

    assert!(
        matches!(
            result,
            Err(update_nft_metadata::UpdateNftMetadataError::InvalidMemo(_))
        ),
        "expected InvalidMemo, got {:?}",
        result
    );
    assert_eq!(log_length(pic, controller, collection_canister_id), before);
}

#[test]
fn test_update_nft_metadata_batch_repeated_token_refused() {
    let mut test_env: TestEnv = test_setup_no_limit();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        ..
    } = test_env;
    let token_ids = mint_tokens(pic, controller, collection_canister_id, nft_owner1, 2);
    let before = log_length(pic, controller, collection_canister_id);

    let result = update_nft_metadata_batch(
        pic,
        controller,
        collection_canister_id,
        &update_nft_metadata_batch::Args {
            items: vec![
                Item {
                    token_id: token_ids[0].clone(),
                    metadata: json_data(r#"{"n":"first"}"#),
                },
                Item {
                    token_id: token_ids[1].clone(),
                    metadata: json_data(r#"{"n":"other"}"#),
                },
                Item {
                    token_id: token_ids[0].clone(),
                    metadata: json_data(r#"{"n":"second"}"#),
                },
            ],
            memo: None,
            log: LogMode::Full,
        },
    );

    assert!(
        matches!(
            result,
            Err(update_nft_metadata::UpdateNftMetadataError::StorageCanisterError(ref message))
                if message == "duplicate token id in batch"
        ),
        "expected the duplicate token refusal, got {:?}",
        result
    );
    assert_eq!(
        log_length(pic, controller, collection_canister_id),
        before,
        "the refused batch wrote no block"
    );
    assert_eq!(
        token_metadata(pic, controller, collection_canister_id, &token_ids[0]),
        Some(json_data(r#"{"n":0}"#))
    );
    assert_eq!(
        token_metadata(pic, controller, collection_canister_id, &token_ids[1]),
        Some(json_data(r#"{"n":1}"#)),
        "the item between the two repeats was not written either"
    );
}
