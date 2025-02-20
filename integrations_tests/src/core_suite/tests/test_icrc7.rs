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
fn test_icrc7_token_metadata_simple() {
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
            new_metadata.insert("test3".to_string(), Value::Blob(ByteBuf::from(logo_data.clone())));

            let update_nft_metadata_args = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: Some("test1".to_string()),
                token_description: Some("description".to_string()),
                token_logo: Some("logo".to_string()),
                token_metadata: Some(new_metadata),
            };

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args
            );

            pic.tick();

            let metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()]
            );

            assert_eq!(metadata[0].clone().unwrap()[0].0, "Description".to_string());
            assert_eq!(metadata[0].clone().unwrap()[0].1, Value::Text("description".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[1].0, "Logo".to_string());
            assert_eq!(metadata[0].clone().unwrap()[1].1, Value::Text("logo".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[2].0, "Name".to_string());
            assert_eq!(metadata[0].clone().unwrap()[2].1, Value::Text("test1".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[3].0, "Symbol".to_string());
            assert_eq!(metadata[0].clone().unwrap()[3].1, Value::Text("test1".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[4].0, "test1".to_string());
            assert_eq!(metadata[0].clone().unwrap()[4].1, Value::Text("test1".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[5].0, "test2".to_string());
            assert_eq!(metadata[0].clone().unwrap()[5].1, Value::Nat(Nat::from(1 as u64)));
            assert_eq!(metadata[0].clone().unwrap()[6].0, "test3".to_string());
            assert_eq!(metadata[0].clone().unwrap()[6].1, Value::Blob(ByteBuf::from(logo_data)));
            assert_eq!(metadata[0].clone().unwrap().len(), 7);
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_token_metadata_multiple_insert() {
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
            new_metadata.insert("test3".to_string(), Value::Blob(ByteBuf::from(logo_data.clone())));

            let update_nft_metadata_args = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: Some("test1".to_string()),
                token_description: Some("description".to_string()),
                token_logo: Some("logo".to_string()),
                token_metadata: Some(new_metadata),
            };

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args
            );

            pic.tick();

            let metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()]
            );

            assert_eq!(metadata[0].clone().unwrap()[0].0, "Description".to_string());
            assert_eq!(metadata[0].clone().unwrap()[0].1, Value::Text("description".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[1].0, "Logo".to_string());
            assert_eq!(metadata[0].clone().unwrap()[1].1, Value::Text("logo".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[2].0, "Name".to_string());
            assert_eq!(metadata[0].clone().unwrap()[2].1, Value::Text("test1".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[3].0, "Symbol".to_string());
            assert_eq!(metadata[0].clone().unwrap()[3].1, Value::Text("test1".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[4].0, "test1".to_string());
            assert_eq!(metadata[0].clone().unwrap()[4].1, Value::Text("test1".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[5].0, "test2".to_string());
            assert_eq!(metadata[0].clone().unwrap()[5].1, Value::Nat(Nat::from(1 as u64)));
            assert_eq!(metadata[0].clone().unwrap()[6].0, "test3".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[6].1,
                Value::Blob(ByteBuf::from(logo_data.clone()))
            );
            assert_eq!(metadata[0].clone().unwrap().len(), 7);

            let mut new_metadata_2: HashMap<String, Value> = HashMap::new();
            new_metadata_2.insert("test4".to_string(), Value::Text("test4".to_string()));
            new_metadata_2.insert("test5".to_string(), Value::Nat(Nat::from(2 as u64)));

            let update_nft_metadata_args_2 = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: None,
                token_description: None,
                token_logo: None,
                token_metadata: Some(new_metadata_2),
            };

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args_2
            );

            let metadata_2 = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()]
            );

            println!("metadata_2: {:?}", metadata_2);

            assert_eq!(metadata[0].clone().unwrap()[0].0, "Description".to_string());
            assert_eq!(metadata[0].clone().unwrap()[0].1, Value::Text("description".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[1].0, "Logo".to_string());
            assert_eq!(metadata[0].clone().unwrap()[1].1, Value::Text("logo".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[2].0, "Name".to_string());
            assert_eq!(metadata[0].clone().unwrap()[2].1, Value::Text("test1".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[3].0, "Symbol".to_string());
            assert_eq!(metadata[0].clone().unwrap()[3].1, Value::Text("test1".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[4].0, "test1".to_string());
            assert_eq!(metadata[0].clone().unwrap()[4].1, Value::Text("test1".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[5].0, "test2".to_string());
            assert_eq!(metadata[0].clone().unwrap()[5].1, Value::Nat(Nat::from(1 as u64)));
            assert_eq!(metadata[0].clone().unwrap()[6].0, "test3".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[6].1,
                Value::Blob(ByteBuf::from(logo_data.clone()))
            );
            assert_eq!(metadata_2[0].clone().unwrap()[7].0, "test4".to_string());
            assert_eq!(metadata_2[0].clone().unwrap()[7].1, Value::Text("test4".to_string()));
            assert_eq!(metadata_2[0].clone().unwrap()[8].0, "test5".to_string());
            assert_eq!(metadata_2[0].clone().unwrap()[8].1, Value::Nat(Nat::from(2 as u64)));
            assert_eq!(metadata_2[0].clone().unwrap().len(), 9);
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_token_metadata_multiple_insert_dup_name() {
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

            let update_nft_metadata_args = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: Some("test1".to_string()),
                token_description: Some("description".to_string()),
                token_logo: Some("logo".to_string()),
                token_metadata: Some(new_metadata),
            };

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args
            );

            pic.tick();

            let metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()]
            );

            println!("metadata: {:?}", metadata);
            assert_eq!(metadata[0].clone().unwrap()[0].0, "Description".to_string());
            assert_eq!(metadata[0].clone().unwrap()[0].1, Value::Text("description".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[1].0, "Logo".to_string());
            assert_eq!(metadata[0].clone().unwrap()[1].1, Value::Text("logo".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[2].0, "Name".to_string());
            assert_eq!(metadata[0].clone().unwrap()[2].1, Value::Text("test1".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[3].0, "Symbol".to_string());
            assert_eq!(metadata[0].clone().unwrap()[3].1, Value::Text("test1".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[4].0, "test1".to_string());
            assert_eq!(metadata[0].clone().unwrap()[4].1, Value::Text("test1".to_string()));
            assert_eq!(metadata[0].clone().unwrap()[5].0, "test2".to_string());
            assert_eq!(metadata[0].clone().unwrap()[5].1, Value::Nat(Nat::from(1 as u64)));
            assert_eq!(metadata[0].clone().unwrap().len(), 6);

            let mut new_metadata_2: HashMap<String, Value> = HashMap::new();
            new_metadata_2.insert("test1".to_string(), Value::Text("test4".to_string()));
            new_metadata_2.insert("test2".to_string(), Value::Nat(Nat::from(2 as u64)));

            let update_nft_metadata_args_2 = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: None,
                token_description: None,
                token_logo: None,
                token_metadata: Some(new_metadata_2),
            };

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args_2
            );

            let metadata_2 = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()]
            );

            println!("metadata_2: {:?}", metadata_2);

            assert_eq!(metadata_2[0].clone().unwrap()[0].0, "Description".to_string());
            assert_eq!(metadata_2[0].clone().unwrap()[0].1, Value::Text("description".to_string()));
            assert_eq!(metadata_2[0].clone().unwrap()[1].0, "Logo".to_string());
            assert_eq!(metadata_2[0].clone().unwrap()[1].1, Value::Text("logo".to_string()));
            assert_eq!(metadata_2[0].clone().unwrap()[2].0, "Name".to_string());
            assert_eq!(metadata_2[0].clone().unwrap()[2].1, Value::Text("test1".to_string()));
            assert_eq!(metadata_2[0].clone().unwrap()[3].0, "Symbol".to_string());
            assert_eq!(metadata_2[0].clone().unwrap()[3].1, Value::Text("test1".to_string()));
            assert_eq!(metadata_2[0].clone().unwrap()[4].0, "test1".to_string());
            assert_eq!(metadata_2[0].clone().unwrap()[4].1, Value::Text("test4".to_string()));
            assert_eq!(metadata_2[0].clone().unwrap()[5].0, "test2".to_string());
            assert_eq!(metadata_2[0].clone().unwrap()[5].1, Value::Nat(Nat::from(2 as u64)));
            assert_eq!(metadata_2[0].clone().unwrap().len(), 6);
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_token_metadata_multiple_insert_big_file() {
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
            let logo_data = include_bytes!("../assets/sbl_hero_1080_1.mp4").to_vec();
            new_metadata.insert("test3".to_string(), Value::Blob(ByteBuf::from(logo_data.clone())));

            let update_nft_metadata_args = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: Some("test1".to_string()),
                token_description: Some("description".to_string()),
                token_logo: Some("logo".to_string()),
                token_metadata: Some(new_metadata),
            };

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args
            );

            pic.tick();

            let metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()]
            );

            // println!("metadata: {:?}", metadata);
            // assert_eq!(metadata[0].clone().unwrap()[0].0, "Description".to_string());
            // assert_eq!(metadata[0].clone().unwrap()[0].1, Value::Text("description".to_string()));
            // assert_eq!(metadata[0].clone().unwrap()[1].0, "Logo".to_string());
            // assert_eq!(metadata[0].clone().unwrap()[1].1, Value::Text("logo".to_string()));
            // assert_eq!(metadata[0].clone().unwrap()[2].0, "Name".to_string());
            // assert_eq!(metadata[0].clone().unwrap()[2].1, Value::Text("test1".to_string()));
            // assert_eq!(metadata[0].clone().unwrap()[3].0, "Symbol".to_string());
            // assert_eq!(metadata[0].clone().unwrap()[3].1, Value::Text("test1".to_string()));
            // assert_eq!(metadata[0].clone().unwrap()[4].0, "test1".to_string());
            // assert_eq!(metadata[0].clone().unwrap()[4].1, Value::Text("test1".to_string()));
            // assert_eq!(metadata[0].clone().unwrap()[5].0, "test2".to_string());
            // assert_eq!(metadata[0].clone().unwrap()[5].1, Value::Nat(Nat::from(1 as u64)));
            // assert_eq!(metadata[0].clone().unwrap().len(), 6);
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}
