use crate::client::core_nft::{
    icrc7_name,
    icrc7_symbol,
    icrc7_description,
    icrc7_logo,
    icrc7_total_supply,
    icrc7_supply_cap,
    icrc7_max_query_batch_size,
    icrc7_max_update_batch_size,
    icrc7_default_take_value,
    icrc7_max_take_value,
    icrc7_max_memo_size,
    icrc7_atomic_batch_transfers,
    icrc7_tx_window,
    icrc7_permitted_drift,
    icrc7_token_metadata,
    icrc7_owner_of,
    icrc7_balance_of,
    icrc7_tokens,
    icrc7_tokens_of,
    icrc7_transfer,
    mint,
    update_minting_authorities,
    update_nft_metadata,
};
use candid::Nat;
use core_nft::types::update_nft_metadata;
use icrc_ledger_types::icrc::generic_value::Hash;
use serde::Serialize;
use serde_bytes::ByteBuf;
use std::collections::HashMap;
use std::time::Duration;
use icrc_ledger_types::icrc1::account::Account;
use crate::utils::mint_nft;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;

use crate::core_suite::setup::setup::TestEnv;
use crate::{ core_suite::setup::default_test_setup, utils::tick_n_blocks };

#[test]
fn test_icrc7_name() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

    let name = icrc7_name(pic, controller, collection_canister_id, &());

    println!("name: {:?}", name);
}

#[test]
fn test_icrc7_symbol() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

    let symbol = icrc7_symbol(pic, controller, collection_canister_id, &());

    println!("symbol: {:?}", symbol);
}

#[test]
fn test_icrc7_total_supply() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

    let total_supply = icrc7_total_supply(pic, controller, collection_canister_id, &());

    println!("total_supply: {:?}", total_supply);
    assert!(total_supply == Nat::from(0 as u64));

    let _ = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id
    );

    let total_supply_2: Nat = icrc7_total_supply(pic, controller, collection_canister_id, &());

    println!("total_supply_2: {:?}", total_supply_2);
    assert!(total_supply_2 == Nat::from(1 as u64));

    let _ = mint_nft(
        pic,
        "test2".to_string(),
        Account {
            owner: nft_owner2,
            subaccount: None,
        },
        controller,
        collection_canister_id
    );

    let total_supply_3: Nat = icrc7_total_supply(pic, controller, collection_canister_id, &());

    println!("total_supply_3: {:?}", total_supply_3);
    assert!(total_supply_3 == Nat::from(2 as u64));
}

#[test]
fn test_icrc7_token_metadata() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv { ref mut pic, collection_canister_id, controller, nft_owner1, nft_owner2 } =
        test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id
    );

    match mint_return {
        Ok(token_id) => {
            let mut new_metadata: HashMap<String, Value> = HashMap::new();
            new_metadata.insert("test1".to_string(), Value::Text("test1".to_string()));
            new_metadata.insert("test2".to_string(), Value::Nat(Nat::from(1 as u64)));
            let logo_data = include_bytes!("../assets/logo2.min-3f9527e7.svg").to_vec();
            new_metadata.insert("test3".to_string(), Value::Blob(ByteBuf::from(logo_data)));

            let update_nft_metadata_args = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: Some("test1".to_string()),
                token_description: Some("description".to_string()),
                token_logo: Some("logo".to_string()),
                token_metadata: Some(new_metadata),
            };

            let update_nft_metadata_result = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args
            );

            pic.tick();

            println!("update_nft_metadata_result: {:?}", update_nft_metadata_result);
            let metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()]
            );

            println!("metadata: {:?}", metadata);
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
        }
    }
}
