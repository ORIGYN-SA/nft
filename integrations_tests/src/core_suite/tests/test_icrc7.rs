use crate::client::core_nft::{
    icrc3_get_blocks, icrc7_atomic_batch_transfers, icrc7_balance_of, icrc7_collection_metadata,
    icrc7_default_take_value, icrc7_description, icrc7_logo, icrc7_max_memo_size,
    icrc7_max_query_batch_size, icrc7_max_take_value, icrc7_max_update_batch_size, icrc7_name,
    icrc7_owner_of, icrc7_permitted_drift, icrc7_supply_cap, icrc7_symbol, icrc7_token_metadata,
    icrc7_total_supply, icrc7_transfer, icrc7_tx_window, update_nft_metadata,
};
use crate::core_suite::setup::setup::TestEnv;
use crate::utils::{
    extract_metadata_file_path, fetch_metadata_json, mint_nft, random_principal, setup_http_client,
    upload_metadata,
};
use bytes::Bytes;
use candid::{Encode, Nat, Principal};
use core_nft::types::icrc7;
use core_nft::types::update_nft_metadata;
use http::Request;
use http_body_util::BodyExt;
use ic_agent::Agent;
use ic_http_gateway::{HttpGatewayClient, HttpGatewayRequestArgs};
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc3::blocks::GetBlocksRequest;
use serde_bytes::ByteBuf;
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::{Duration, UNIX_EPOCH};
use url::Url;

use crate::{
    core_suite::setup::{default_test_setup, test_setup_atomic_batch_transfers},
    utils::tick_n_blocks,
};

#[test]
fn test_icrc7_name() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let name = icrc7_name(pic, controller, collection_canister_id, &());

    println!("name: {:?}", name);
}

#[test]
fn test_icrc7_symbol() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let symbol = icrc7_symbol(pic, controller, collection_canister_id, &());

    println!("symbol: {:?}", symbol);
}

#[test]
fn test_icrc7_total_supply() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let total_supply = icrc7_total_supply(pic, controller, collection_canister_id, &());

    tick_n_blocks(pic, 10);

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
        collection_canister_id,
    );

    tick_n_blocks(pic, 10);

    let total_supply_2: Nat = icrc7_total_supply(pic, controller, collection_canister_id, &());

    tick_n_blocks(pic, 10);

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
        collection_canister_id,
    );

    tick_n_blocks(pic, 10);

    let total_supply_3: Nat = icrc7_total_supply(pic, controller, collection_canister_id, &());

    println!("total_supply_3: {:?}", total_supply_3);
    assert!(total_supply_3 == Nat::from(2 as u64));
}

#[test]
fn test_icrc7_token_metadata_simple() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(token_id) => {
            let metadata_json = json!({
                "description": "Test NFT description",
                "name": "Test NFT",
                "attributes": [
                    {
                        "trait_type": "test1",
                        "value": "test11"
                    },
                    {
                        "trait_type": "test2",
                        "value": "test22"
                    },
                    {
                        "trait_type": "test3",
                        "value": 2
                    }
                ]
            });

            let metadata_url =
                upload_metadata(pic, controller, collection_canister_id, metadata_json).unwrap();

            tick_n_blocks(pic, 10);

            let update_nft_metadata_args = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: Some("test1".to_string()),
                token_metadata_url: metadata_url.to_string(),
            };

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args,
            );

            tick_n_blocks(pic, 10);

            let metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            println!("metadata: {:?}", metadata);

            assert_eq!(
                metadata[0].clone().unwrap()[0].0,
                "icrc97:metadata".to_string()
            );
            assert_eq!(
                metadata[0].clone().unwrap()[0].1,
                Value::Array(vec![Value::Text(metadata_url.to_string()),])
            );

            let (rt, http_gateway) = setup_http_client(pic);
            let metadata_file_path = extract_metadata_file_path(&metadata_url);
            let parsed_metadata = fetch_metadata_json(
                &rt,
                &http_gateway,
                collection_canister_id,
                &metadata_file_path,
            );
            assert_eq!(
                parsed_metadata
                    .get("description")
                    .unwrap()
                    .as_str()
                    .unwrap(),
                "Test NFT description"
            );
            assert_eq!(
                parsed_metadata.get("name").unwrap().as_str().unwrap(),
                "Test NFT"
            );

            let attributes = parsed_metadata
                .get("attributes")
                .unwrap()
                .as_array()
                .unwrap();
            assert_eq!(
                attributes[0].get("trait_type").unwrap().as_str().unwrap(),
                "test1"
            );
            assert_eq!(
                attributes[0].get("value").unwrap().as_str().unwrap(),
                "test11"
            );

            println!("Verification of the JSON file metadata successful!");
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

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(token_id) => {
            let metadata_json_1 = json!({
                "description": "First metadata description",
                "name": "First NFT",
                "attributes": [
                    {
                        "trait_type": "test1",
                        "value": "test1"
                    },
                    {
                        "trait_type": "test2",
                        "value": 1
                    },
                    {
                        "trait_type": "test3",
                        "value": "blob_data"
                    }
                ]
            });

            let metadata_url_1 =
                upload_metadata(pic, controller, collection_canister_id, metadata_json_1).unwrap();

            tick_n_blocks(pic, 10);

            let update_nft_metadata_args = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: Some("test1".to_string()),
                token_metadata_url: metadata_url_1.to_string(),
            };

            let ret = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args,
            );

            println!("ret: {:?}", ret);

            tick_n_blocks(pic, 10);

            println!("token_id: {:?}", token_id);

            let metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            println!("metadata: {:?}", metadata);

            assert_eq!(
                metadata[0].clone().unwrap()[0].0,
                "icrc97:metadata".to_string()
            );
            assert_eq!(
                metadata[0].clone().unwrap()[0].1,
                Value::Array(vec![Value::Text(metadata_url_1.to_string())])
            );

            let metadata_json_2 = json!({
                "description": "Second metadata description",
                "name": "Second NFT",
                "attributes": [
                    {
                        "trait_type": "test4",
                        "value": "test4"
                    },
                    {
                        "trait_type": "test5",
                        "value": 2
                    }
                ]
            });

            let metadata_url_2 =
                upload_metadata(pic, controller, collection_canister_id, metadata_json_2).unwrap();

            let update_nft_metadata_args_2 = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: None,
                token_metadata_url: metadata_url_2.to_string(),
            };

            println!("update_nft_metadata_args_2");

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args_2,
            );

            tick_n_blocks(pic, 10);

            let metadata_2 = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            println!("metadata_2: {:?}", metadata_2);

            assert_eq!(
                metadata_2[0].clone().unwrap()[0].0,
                "icrc97:metadata".to_string()
            );
            assert_eq!(
                metadata_2[0].clone().unwrap()[0].1,
                Value::Array(vec![Value::Text(metadata_url_2.to_string())])
            );

            let (rt, http_gateway) = setup_http_client(pic);
            let metadata_file_path = extract_metadata_file_path(&metadata_url_2);
            let parsed_metadata = fetch_metadata_json(
                &rt,
                &http_gateway,
                collection_canister_id,
                &metadata_file_path,
            );
            assert_eq!(
                parsed_metadata
                    .get("description")
                    .unwrap()
                    .as_str()
                    .unwrap(),
                "Second metadata description"
            );
            assert_eq!(
                parsed_metadata.get("name").unwrap().as_str().unwrap(),
                "Second NFT"
            );

            println!("Verification of multiple insert metadata successful!");
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

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    tick_n_blocks(pic, 5);

    match mint_return {
        Ok(token_id) => {
            let metadata_json_1 = json!({
                "description": "Initial metadata",
                "name": "test1",
                "attributes": [
                    {
                        "trait_type": "test1",
                        "value": "test1"
                    },
                    {
                        "trait_type": "test2",
                        "value": 1
                    }
                ]
            });

            let metadata_url_1 =
                upload_metadata(pic, controller, collection_canister_id, metadata_json_1).unwrap();

            let update_nft_metadata_args = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: Some("test1".to_string()),
                token_metadata_url: metadata_url_1.to_string(),
            };

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args,
            );

            tick_n_blocks(pic, 5);

            let metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            println!("metadata: {:?}", metadata);
            assert_eq!(
                metadata[0].clone().unwrap()[0].0,
                "icrc97:metadata".to_string()
            );
            assert_eq!(
                metadata[0].clone().unwrap()[0].1,
                Value::Array(vec![Value::Text(metadata_url_1.to_string())])
            );

            let metadata_json_2 = json!({
                "description": "Updated metadata with duplicated traits",
                "name": "test1",
                "attributes": [
                    {
                        "trait_type": "test1",
                        "value": "test4"
                    },
                    {
                        "trait_type": "test2",
                        "value": 2
                    }
                ]
            });

            let metadata_url_2 =
                upload_metadata(pic, controller, collection_canister_id, metadata_json_2).unwrap();

            let update_nft_metadata_args_2 = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: None,
                token_metadata_url: metadata_url_2.to_string(),
            };

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args_2,
            );
            tick_n_blocks(pic, 5);

            let metadata_2 = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            println!("metadata_2: {:?}", metadata_2);

            assert_eq!(
                metadata_2[0].clone().unwrap()[0].0,
                "icrc97:metadata".to_string()
            );
            assert_eq!(
                metadata_2[0].clone().unwrap()[0].1,
                Value::Array(vec![Value::Text(metadata_url_2.to_string())])
            );

            let (rt, http_gateway) = setup_http_client(pic);
            let metadata_file_path = extract_metadata_file_path(&metadata_url_2);
            let parsed_metadata = fetch_metadata_json(
                &rt,
                &http_gateway,
                collection_canister_id,
                &metadata_file_path,
            );

            assert_eq!(
                parsed_metadata
                    .get("description")
                    .unwrap()
                    .as_str()
                    .unwrap(),
                "Updated metadata with duplicated traits"
            );
            assert_eq!(
                parsed_metadata.get("name").unwrap().as_str().unwrap(),
                "test1"
            );

            let attributes = parsed_metadata
                .get("attributes")
                .unwrap()
                .as_array()
                .unwrap();
            assert_eq!(
                attributes[0].get("trait_type").unwrap().as_str().unwrap(),
                "test1"
            );
            assert_eq!(
                attributes[0].get("value").unwrap().as_str().unwrap(),
                "test4"
            );

            assert_eq!(
                attributes[1].get("trait_type").unwrap().as_str().unwrap(),
                "test2"
            );
            assert_eq!(attributes[1].get("value").unwrap().as_i64().unwrap(), 2);

            println!("Verification of duplicate name metadata successful!");
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_supply_cap() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let supply_cap = icrc7_supply_cap(pic, controller, collection_canister_id, &())
        .unwrap_or(Nat::from(100u64))
        .0
        .try_into()
        .unwrap_or(100u64);

    for i in 0..supply_cap {
        println!("Minting token: {}", i);
        let mint_return = mint_nft(
            pic,
            format!("test{}", i),
            Account {
                owner: nft_owner1,
                subaccount: None,
            },
            controller,
            collection_canister_id,
        );
        pic.advance_time(Duration::from_secs(1));
        tick_n_blocks(pic, 5);
        assert!(mint_return.is_ok());
    }

    let mint_return = mint_nft(
        pic,
        "test_overflow".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );
    assert!(mint_return.is_err());

    let total_supply = icrc7_total_supply(pic, controller, collection_canister_id, &());
    assert_eq!(total_supply, Nat::from(supply_cap));
}

#[test]
fn test_icrc7_transfer_with_metadata_updates() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );
    tick_n_blocks(pic, 5);

    match mint_return {
        Ok(token_id) => {
            let current_time = pic.get_time().as_nanos_since_unix_epoch();

            let metadata_json_1 = json!({
                "description": "Pretransfer metadata",
                "name": "test1",
                "attributes": [
                    {
                        "trait_type": "test1",
                        "value": "test1"
                    },
                    {
                        "trait_type": "test2",
                        "value": 1
                    }
                ]
            });

            let metadata_url_1 =
                upload_metadata(pic, controller, collection_canister_id, metadata_json_1).unwrap();

            let update_nft_metadata_args = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: Some("test1".to_string()),
                token_metadata_url: metadata_url_1.to_string(),
            };

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args,
            );
            tick_n_blocks(pic, 5);

            let transfer_args = vec![icrc7::TransferArg {
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                from_subaccount: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response =
                icrc7_transfer(pic, nft_owner1, collection_canister_id, &transfer_args);
            assert!(
                transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok()
            );
            tick_n_blocks(pic, 5);

            let metadata_json_2 = json!({
                "description": "Posttransfer metadata",
                "name": "test2",
                "attributes": [
                    {
                        "trait_type": "test3",
                        "value": "test3"
                    },
                    {
                        "trait_type": "test4",
                        "value": 2
                    }
                ]
            });

            let metadata_url_2 =
                upload_metadata(pic, controller, collection_canister_id, metadata_json_2).unwrap();

            let update_nft_metadata_args_2 = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: Some("test2".to_string()),
                token_metadata_url: metadata_url_2.to_string(),
            };

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args_2,
            );
            tick_n_blocks(pic, 5);

            let owner_of = icrc7_owner_of(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );
            assert_eq!(
                owner_of[0],
                Some(Account {
                    owner: nft_owner2,
                    subaccount: None
                })
            );
            tick_n_blocks(pic, 5);

            let metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );
            assert_eq!(
                metadata[0].clone().unwrap()[0].0,
                "icrc97:metadata".to_string()
            );
            assert_eq!(
                metadata[0].clone().unwrap()[0].1,
                Value::Array(vec![Value::Text(metadata_url_2.to_string())])
            );
            tick_n_blocks(pic, 5);

            let (rt, http_gateway) = setup_http_client(pic);
            let metadata_file_path = extract_metadata_file_path(&metadata_url_2);
            let parsed_metadata = fetch_metadata_json(
                &rt,
                &http_gateway,
                collection_canister_id,
                &metadata_file_path,
            );

            assert_eq!(
                parsed_metadata
                    .get("description")
                    .unwrap()
                    .as_str()
                    .unwrap(),
                "Posttransfer metadata"
            );
            assert_eq!(
                parsed_metadata.get("name").unwrap().as_str().unwrap(),
                "test2"
            );

            let attributes = parsed_metadata
                .get("attributes")
                .unwrap()
                .as_array()
                .unwrap();
            assert_eq!(
                attributes[0].get("trait_type").unwrap().as_str().unwrap(),
                "test3"
            );
            assert_eq!(
                attributes[0].get("value").unwrap().as_str().unwrap(),
                "test3"
            );

            assert_eq!(
                attributes[1].get("trait_type").unwrap().as_str().unwrap(),
                "test4"
            );
            assert_eq!(attributes[1].get("value").unwrap().as_i64().unwrap(), 2);

            println!("Verification of metadata updates with transfer successful!");
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_max_memo_size() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let max_memo_size = icrc7_max_memo_size(pic, controller, collection_canister_id, &());

    println!("max_memo_size: {:?}", max_memo_size);
    assert!(max_memo_size == None);
}

#[test]
fn test_icrc7_atomic_batch_transfers() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let atomic_batch_transfers =
        icrc7_atomic_batch_transfers(pic, controller, collection_canister_id, &());

    println!("atomic_batch_transfers: {:?}", atomic_batch_transfers);
    assert!(atomic_batch_transfers == None);
}

#[test]
fn test_icrc7_tx_window() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let tx_window = icrc7_tx_window(pic, controller, collection_canister_id, &());

    println!("tx_window: {:?}", tx_window);
    assert!(tx_window == None);
}

#[test]
fn test_icrc7_permitted_drift() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let permitted_drift = icrc7_permitted_drift(pic, controller, collection_canister_id, &());

    println!("permitted_drift: {:?}", permitted_drift);
    assert!(permitted_drift == None);
}

#[test]
fn test_icrc7_transfer() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(token_id) => {
            let transfer_args = icrc7::TransferArg {
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                from_subaccount: None,
                created_at_time: None,
            };

            let transfer_response = icrc7_transfer(
                pic,
                nft_owner1,
                collection_canister_id,
                &vec![transfer_args],
            );

            println!("transfer_response: {:?}", transfer_response);
            assert!(
                transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok()
            );

            let owner_of = icrc7_owner_of(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );
            assert_eq!(
                owner_of[0],
                Some(Account {
                    owner: nft_owner2,
                    subaccount: None
                })
            );
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_collection_metadata() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let metadata = icrc7_collection_metadata(pic, controller, collection_canister_id, &());
    println!("metadata: {:?}", metadata);

    assert!(metadata
        .iter()
        .any(|(key, value)| { key == "icrc7:symbol" && matches!(value, Value::Text(_)) }));
    assert!(metadata
        .iter()
        .any(|(key, value)| { key == "icrc7:name" && matches!(value, Value::Text(_)) }));
    assert!(metadata
        .iter()
        .any(|(key, value)| { key == "icrc7:total_supply" && matches!(value, Value::Nat(_)) }));

    let total_supply = metadata
        .iter()
        .find(|(key, _)| key == "icrc7:total_supply")
        .map(|(_, value)| match value {
            Value::Nat(n) => n.clone(),
            _ => Nat::from(0u64),
        })
        .unwrap_or(Nat::from(0u64));
    assert_eq!(total_supply, Nat::from(0u64));

    let _ = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    let updated_metadata = icrc7_collection_metadata(pic, controller, collection_canister_id, &());
    let updated_total_supply = updated_metadata
        .iter()
        .find(|(key, _)| key == "icrc7:total_supply")
        .map(|(_, value)| match value {
            Value::Nat(n) => n.clone(),
            _ => Nat::from(0u64),
        })
        .unwrap_or(Nat::from(0u64));
    assert_eq!(updated_total_supply, Nat::from(1u64));

    let mut sorted_metadata = metadata.clone();
    sorted_metadata.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(metadata, sorted_metadata);
}

#[test]
fn test_icrc3_logs_metadata_updates() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test_metadata_logs".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    tick_n_blocks(pic, 5);

    match mint_return {
        Ok(token_id) => {
            let blocks_before = icrc3_get_blocks(
                pic,
                controller,
                collection_canister_id,
                &vec![GetBlocksRequest {
                    start: Nat::from(0u64),
                    length: Nat::from(10u64),
                }],
            );

            println!("blocks_before: {:?}", blocks_before);
            let initial_log_length = blocks_before.log_length.clone();

            let metadata_json = json!({
                "description": "Updated NFT description for ICRC3 test",
                "name": "Updated NFT",
                "attributes": [
                    {
                        "trait_type": "icrc3_test",
                        "value": "updated_value"
                    },
                    {
                        "trait_type": "version",
                        "value": 2
                    }
                ]
            });

            let metadata_url =
                upload_metadata(pic, controller, collection_canister_id, metadata_json).unwrap();

            tick_n_blocks(pic, 10);

            let update_nft_metadata_args = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: Some("updated_test".to_string()),
                token_metadata_url: metadata_url.to_string(),
            };

            let update_result = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args,
            );

            assert!(
                update_result.is_ok(),
                "Failed to update NFT metadata: {:?}",
                update_result
            );

            tick_n_blocks(pic, 10);

            let blocks_after = icrc3_get_blocks(
                pic,
                controller,
                collection_canister_id,
                &vec![GetBlocksRequest {
                    start: Nat::from(0u64),
                    length: Nat::from(20u64),
                }],
            );

            println!("blocks_after: {:?}", blocks_after);
            println!("initial_log_length: {:?}", initial_log_length);
            println!("final_log_length: {:?}", blocks_after.log_length);

            assert!(
                blocks_after.log_length > initial_log_length,
                "Log length should increase after metadata update"
            );

            let mut found_update_block = false;

            for block in &blocks_after.blocks {
                match &block.block {
                    Value::Map(map) => {
                        if let Some(Value::Text(btype)) = map.get("btype") {
                            println!("Found block type: {}", btype);
                            if btype == "7update_token" {
                                found_update_block = true;

                                if let Some(Value::Map(tx_map)) = map.get("tx") {
                                    assert!(
                                        tx_map.contains_key("token_id"),
                                        "Update transaction should contain token_id"
                                    );

                                    if let Some(Value::Nat(tx_token_id)) = tx_map.get("token_id") {
                                        assert_eq!(
                                            *tx_token_id, token_id,
                                            "Token ID in transaction should match updated token"
                                        );
                                    }
                                }
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }

            if !found_update_block {
                for archived_block_info in &blocks_after.archived_blocks {
                    let archived_blocks = icrc3_get_blocks(
                        pic,
                        controller,
                        archived_block_info.callback.canister_id,
                        &archived_block_info.args,
                    );

                    for block in &archived_blocks.blocks {
                        match &block.block {
                            Value::Map(map) => {
                                if let Some(Value::Text(btype)) = map.get("btype") {
                                    println!("Found archived block type: {}", btype);
                                    if btype == "7update_token" {
                                        found_update_block = true;

                                        if let Some(Value::Map(tx_map)) = map.get("tx") {
                                            if let Some(Value::Nat(tx_token_id)) =
                                                tx_map.get("token_id")
                                            {
                                                assert_eq!(
                                                    *tx_token_id, token_id,
                                                    "Token ID in archived transaction should match updated token"
                                                );
                                            }
                                        }

                                        println!("Found valid metadata update transaction in archived ICRC3 logs");
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    if found_update_block {
                        break;
                    }
                }
            }

            assert!(
                found_update_block,
                "Should find a 7update_token block in ICRC3 logs after metadata update"
            );

            let final_metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            assert_eq!(
                final_metadata[0].clone().unwrap()[0].0,
                "icrc97:metadata".to_string()
            );
            assert_eq!(
                final_metadata[0].clone().unwrap()[0].1,
                Value::Array(vec![Value::Text(metadata_url.to_string())])
            );
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_max_take_value() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let max_take_value = icrc7_max_take_value(pic, controller, collection_canister_id, &());

    println!("max_take_value: {:?}", max_take_value);
    assert!(max_take_value == None);
}

#[test]
fn test_icrc7_description() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let description = icrc7_description(pic, controller, collection_canister_id, &());

    println!("description: {:?}", description);
    assert!(description == None);
}

#[test]
fn test_icrc7_logo() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let logo = icrc7_logo(pic, controller, collection_canister_id, &());

    println!("logo: {:?}", logo);
    assert!(logo == None);
}

#[test]
fn test_icrc7_tokens() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let tokens: core_nft::types::icrc7::icrc7_tokens::Response =
        crate::client::pocket::unwrap_response(pic.query_call(
            collection_canister_id,
            controller,
            "icrc7_tokens",
            Encode!(&(), &()).unwrap(),
        ));

    println!("tokens: {:?}", tokens);
    assert!(tokens.is_empty());

    let _ = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    tick_n_blocks(pic, 5);

    let tokens: core_nft::types::icrc7::icrc7_tokens::Response =
        crate::client::pocket::unwrap_response(pic.query_call(
            collection_canister_id,
            controller,
            "icrc7_tokens",
            Encode!(&(), &()).unwrap(),
        ));

    println!("tokens: {:?}", tokens);
    assert_eq!(tokens.len(), 1);

    let _ = mint_nft(
        pic,
        "test2".to_string(),
        Account {
            owner: nft_owner2,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    tick_n_blocks(pic, 5);

    let tokens: core_nft::types::icrc7::icrc7_tokens::Response =
        crate::client::pocket::unwrap_response(pic.query_call(
            collection_canister_id,
            controller,
            "icrc7_tokens",
            Encode!(&(), &()).unwrap(),
        ));
    println!("tokens: {:?}", tokens);
    assert_eq!(tokens.len(), 2);
}

#[test]
fn test_icrc7_tokens_of() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let tokens_of_owner1: core_nft::types::icrc7::icrc7_tokens_of::Response =
        crate::client::pocket::unwrap_response(
            pic.query_call(
                collection_canister_id,
                controller,
                "icrc7_tokens_of",
                Encode!(
                    &Account {
                        owner: nft_owner1,
                        subaccount: None,
                    },
                    &(),
                    &()
                )
                .unwrap(),
            ),
        );

    println!("tokens_of_owner1: {:?}", tokens_of_owner1);
    assert!(tokens_of_owner1.is_empty());

    let _ = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    tick_n_blocks(pic, 5);

    let tokens_of_owner1: core_nft::types::icrc7::icrc7_tokens_of::Response =
        crate::client::pocket::unwrap_response(
            pic.query_call(
                collection_canister_id,
                controller,
                "icrc7_tokens_of",
                Encode!(
                    &Account {
                        owner: nft_owner1,
                        subaccount: None,
                    },
                    &(),
                    &()
                )
                .unwrap(),
            ),
        );

    println!("tokens_of_owner1: {:?}", tokens_of_owner1);
    assert_eq!(tokens_of_owner1.len(), 1);

    let _ = mint_nft(
        pic,
        "test2".to_string(),
        Account {
            owner: nft_owner2,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    tick_n_blocks(pic, 5);

    let tokens_of_owner2: core_nft::types::icrc7::icrc7_tokens_of::Response =
        crate::client::pocket::unwrap_response(
            pic.query_call(
                collection_canister_id,
                controller,
                "icrc7_tokens_of",
                Encode!(
                    &Account {
                        owner: nft_owner2,
                        subaccount: None,
                    },
                    &(),
                    &()
                )
                .unwrap(),
            ),
        );

    println!("tokens_of_owner2: {:?}", tokens_of_owner2);
    assert_eq!(tokens_of_owner2.len(), 1);
}

#[test]
fn test_icrc7_balance_of() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let balance_of_owner1 = icrc7_balance_of(
        pic,
        controller,
        collection_canister_id,
        &vec![Account {
            owner: nft_owner1,
            subaccount: None,
        }],
    );
    println!("balance_of_owner1: {:?}", balance_of_owner1);
    assert_eq!(balance_of_owner1[0], Nat::from(0 as u64));

    let _ = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    tick_n_blocks(pic, 5);

    let balance_of_owner1 = icrc7_balance_of(
        pic,
        controller,
        collection_canister_id,
        &vec![Account {
            owner: nft_owner1,
            subaccount: None,
        }],
    );
    println!("balance_of_owner1: {:?}", balance_of_owner1);
    assert_eq!(balance_of_owner1[0], Nat::from(1 as u64));

    let _ = mint_nft(
        pic,
        "test2".to_string(),
        Account {
            owner: nft_owner2,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );
    tick_n_blocks(pic, 5);

    let balance_of_owner2 = icrc7_balance_of(
        pic,
        controller,
        collection_canister_id,
        &vec![Account {
            owner: nft_owner2,
            subaccount: None,
        }],
    );
    println!("balance_of_owner2: {:?}", balance_of_owner2);
    assert_eq!(balance_of_owner2[0], Nat::from(1 as u64));
}

#[test]
fn test_icrc7_transfer_batch() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mut token_ids = Vec::new();
    for i in 0..3 {
        let mint_return = mint_nft(
            pic,
            format!("test{}", i),
            Account {
                owner: nft_owner1,
                subaccount: None,
            },
            controller,
            collection_canister_id,
        );

        tick_n_blocks(pic, 5);

        if let Ok(token_id) = mint_return {
            token_ids.push(token_id);
        }
    }

    let transfer_args: Vec<icrc7::TransferArg> = token_ids
        .iter()
        .map(|token_id| icrc7::TransferArg {
            to: Account {
                owner: nft_owner2,
                subaccount: None,
            },
            token_id: token_id.clone(),
            memo: None,
            from_subaccount: None,
            created_at_time: None,
        })
        .collect();

    let transfer_response = icrc7_transfer(pic, nft_owner1, collection_canister_id, &transfer_args);

    for response in transfer_response {
        assert!(response.is_some() && response.unwrap().is_ok());
    }

    for token_id in token_ids {
        let owner_of = icrc7_owner_of(pic, controller, collection_canister_id, &vec![token_id]);
        assert_eq!(
            owner_of[0],
            Some(Account {
                owner: nft_owner2,
                subaccount: None
            })
        );
        tick_n_blocks(pic, 5);
    }
}

#[test]
fn test_icrc7_transfer_invalid_recipient() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(token_id) => {
            let transfer_args = icrc7::TransferArg {
                to: Account {
                    owner: Principal::anonymous(),
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                from_subaccount: None,
                created_at_time: None,
            };

            let transfer_response = icrc7_transfer(
                pic,
                nft_owner1,
                collection_canister_id,
                &vec![transfer_args],
            );

            println!("transfer_response: {:?}", transfer_response);

            assert!(
                transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_err()
            );
            assert_eq!(
                transfer_response[0].clone().unwrap().err().unwrap(),
                icrc7::icrc7_transfer::TransferError::InvalidRecipient
            );
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_transfer_permitted_drift() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(token_id) => {
            let nanos = pic.get_time().as_nanos_since_unix_epoch();
            let future_time = nanos + 100_000_000;

            let transfer_args = icrc7::TransferArg {
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                from_subaccount: None,
                created_at_time: Some(future_time),
            };

            let transfer_response = icrc7_transfer(
                pic,
                nft_owner1,
                collection_canister_id,
                &vec![transfer_args],
            );

            assert!(
                transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_err()
            );
            assert_eq!(
                transfer_response[0].clone().unwrap().err().unwrap(),
                icrc7::icrc7_transfer::TransferError::CreatedInFuture {
                    ledger_time: Nat::from(nanos),
                }
            );
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_transfer_within_permitted_drift() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(token_id) => {
            let nanos = pic.get_time().as_nanos_since_unix_epoch();
            let future_time = nanos + 10_000_000;
            let transfer_args = icrc7::TransferArg {
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                from_subaccount: None,
                created_at_time: Some(future_time),
            };

            let transfer_response = icrc7_transfer(
                pic,
                nft_owner1,
                collection_canister_id,
                &vec![transfer_args],
            );

            println!("transfer_response: {:?}", transfer_response);

            assert!(
                transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok()
            );

            let owner_of = icrc7_owner_of(pic, controller, collection_canister_id, &vec![token_id]);
            assert_eq!(
                owner_of[0],
                Some(Account {
                    owner: nft_owner2,
                    subaccount: None
                })
            );
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_transfer_too_old() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(token_id) => {
            let nanos = pic.get_time().as_nanos_since_unix_epoch();
            let old_time = nanos - 3_600_000_000_000;
            println!("old_time: {:?}", old_time);
            let transfer_args = icrc7::TransferArg {
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                from_subaccount: None,
                created_at_time: Some(old_time),
            };

            let transfer_response = icrc7_transfer(
                pic,
                nft_owner1,
                collection_canister_id,
                &vec![transfer_args],
            );

            println!("transfer_response: {:?}", transfer_response);

            assert!(
                transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_err()
            );
            assert_eq!(
                transfer_response[0].clone().unwrap().err().unwrap(),
                icrc7::icrc7_transfer::TransferError::TooOld
            );
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_transfer_old_but_valid() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(token_id) => {
            let nanos = pic.get_time().as_nanos_since_unix_epoch();
            let old_time = nanos - 5_000_000_000; // 5 seconds
            println!("old_time: {:?}", old_time);
            let transfer_args = icrc7::TransferArg {
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                from_subaccount: None,
                created_at_time: Some(old_time),
            };

            let transfer_response = icrc7_transfer(
                pic,
                nft_owner1,
                collection_canister_id,
                &vec![transfer_args],
            );

            println!("transfer_response: {:?}", transfer_response);

            assert!(
                transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok()
            );

            let owner_of = icrc7_owner_of(pic, controller, collection_canister_id, &vec![token_id]);
            assert_eq!(
                owner_of[0],
                Some(Account {
                    owner: nft_owner2,
                    subaccount: None
                })
            );
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_transfer_batch_with_memo() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let nft_owner3 = random_principal();

    let mut token_ids = Vec::new();
    for i in 0..3 {
        let mint_return = mint_nft(
            pic,
            format!("test{}", i),
            Account {
                owner: nft_owner1,
                subaccount: None,
            },
            controller,
            collection_canister_id,
        );
        if let Ok(token_id) = mint_return {
            token_ids.push(token_id);
        }

        tick_n_blocks(pic, 5);
    }

    let current_time = pic.get_time().as_nanos_since_unix_epoch();

    let transfer_args: Vec<icrc7::TransferArg> = token_ids
        .iter()
        .enumerate()
        .map(|(i, token_id)| icrc7::TransferArg {
            to: Account {
                owner: nft_owner2,
                subaccount: None,
            },
            token_id: token_id.clone(),
            memo: Some(ByteBuf::from(format!("memo{}", i).as_bytes().to_vec())),
            from_subaccount: None,
            created_at_time: Some(current_time),
        })
        .collect();

    let transfer_response = icrc7_transfer(pic, nft_owner1, collection_canister_id, &transfer_args);

    for response in transfer_response {
        assert!(response.is_some() && response.unwrap().is_ok());
    }

    for token_id in token_ids {
        let owner_of = icrc7_owner_of(pic, controller, collection_canister_id, &vec![token_id]);
        assert_eq!(
            owner_of[0],
            Some(Account {
                owner: nft_owner2,
                subaccount: None
            })
        );

        tick_n_blocks(pic, 5);
    }
}

#[test]
fn test_icrc7_transfer_batch_with_subaccounts() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let nft_owner3 = random_principal();
    let subaccount1 = Some([1u8; 32]);
    let subaccount2 = Some([2u8; 32]);

    let mut token_ids = Vec::new();
    for i in 0..3 {
        let mint_return = mint_nft(
            pic,
            format!("test{}", i),
            Account {
                owner: nft_owner1,
                subaccount: if i % 2 == 0 {
                    subaccount1.clone()
                } else {
                    subaccount2.clone()
                },
            },
            controller,
            collection_canister_id,
        );
        if let Ok(token_id) = mint_return {
            token_ids.push(token_id);
        }

        tick_n_blocks(pic, 5);
    }

    let current_time = pic.get_time().as_nanos_since_unix_epoch();

    let transfer_args: Vec<icrc7::TransferArg> = token_ids
        .iter()
        .enumerate()
        .map(|(i, token_id)| icrc7::TransferArg {
            to: Account {
                owner: nft_owner2,
                subaccount: if i % 2 == 0 {
                    subaccount1.clone()
                } else {
                    subaccount2.clone()
                },
            },
            token_id: token_id.clone(),
            memo: None,
            from_subaccount: if i % 2 == 0 {
                Some(ByteBuf::from(subaccount1.clone().unwrap()))
            } else {
                Some(ByteBuf::from(subaccount2.clone().unwrap()))
            },
            created_at_time: Some(current_time),
        })
        .collect();

    let transfer_response = icrc7_transfer(pic, nft_owner1, collection_canister_id, &transfer_args);

    for response in transfer_response {
        println!("response: {:?}", response);
        assert!(response.is_some() && response.unwrap().is_ok());
    }

    // Verify all tokens are now owned by nft_owner2 with correct subaccounts
    for (i, token_id) in token_ids.iter().enumerate() {
        let owner_of = icrc7_owner_of(
            pic,
            controller,
            collection_canister_id,
            &vec![token_id.clone()],
        );
        assert_eq!(
            owner_of[0],
            Some(Account {
                owner: nft_owner2,
                subaccount: if i % 2 == 0 {
                    subaccount1.clone()
                } else {
                    subaccount2.clone()
                }
            })
        );
        tick_n_blocks(pic, 5);
    }
}

#[test]
fn test_icrc7_transfer_batch_with_time_constraints() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let nft_owner3 = random_principal();

    let mut token_ids = Vec::new();
    for i in 0..3 {
        let mint_return = mint_nft(
            pic,
            format!("test{}", i),
            Account {
                owner: nft_owner1,
                subaccount: None,
            },
            controller,
            collection_canister_id,
        );

        tick_n_blocks(pic, 5);
        if let Ok(token_id) = mint_return {
            token_ids.push(token_id);
        }
    }

    let current_time = pic.get_time().as_nanos_since_unix_epoch();

    let transfer_args: Vec<icrc7::TransferArg> = token_ids
        .iter()
        .enumerate()
        .map(|(i, token_id)| icrc7::TransferArg {
            to: Account {
                owner: nft_owner2,
                subaccount: None,
            },
            token_id: token_id.clone(),
            memo: None,
            from_subaccount: None,
            created_at_time: Some(current_time + (i as u64 * 1_000_000)),
        })
        .collect();

    let transfer_response = icrc7_transfer(pic, nft_owner1, collection_canister_id, &transfer_args);

    for response in transfer_response {
        assert!(response.is_some() && response.unwrap().is_ok());
    }

    for token_id in token_ids {
        let owner_of = icrc7_owner_of(pic, controller, collection_canister_id, &vec![token_id]);
        assert_eq!(
            owner_of[0],
            Some(Account {
                owner: nft_owner2,
                subaccount: None
            })
        );
        tick_n_blocks(pic, 5);
    }
}

#[test]
fn test_icrc7_transfer_with_max_memo_size() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(token_id) => {
            let current_time = pic.get_time().as_nanos_since_unix_epoch();

            let max_memo_size = icrc7_max_memo_size(pic, controller, collection_canister_id, &())
                .unwrap_or(Nat::from(icrc7::DEFAULT_MAX_MEMO_SIZE))
                .0
                .try_into()
                .unwrap_or(icrc7::DEFAULT_MAX_MEMO_SIZE as usize);

            let memo = vec![0u8; max_memo_size];

            let transfer_args = vec![icrc7::TransferArg {
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: Some(ByteBuf::from(memo)),
                from_subaccount: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response =
                icrc7_transfer(pic, nft_owner1, collection_canister_id, &transfer_args);

            assert!(
                transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok()
            );

            let owner_of = icrc7_owner_of(pic, controller, collection_canister_id, &vec![token_id]);
            assert_eq!(
                owner_of[0],
                Some(Account {
                    owner: nft_owner2,
                    subaccount: None
                })
            );
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_transfer_with_supply_cap() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let supply_cap = icrc7_supply_cap(pic, controller, collection_canister_id, &())
        .unwrap_or(Nat::from(100u64))
        .0
        .try_into()
        .unwrap_or(100u64);

    println!("supply_cap: {:?}", supply_cap);

    for i in 0..supply_cap {
        let mint_return = mint_nft(
            pic,
            format!("test{}", i),
            Account {
                owner: nft_owner1,
                subaccount: None,
            },
            controller,
            collection_canister_id,
        );
        assert!(mint_return.is_ok());
        tick_n_blocks(pic, 5);
    }

    let mint_return = mint_nft(
        pic,
        "test_overflow".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );
    assert!(mint_return.is_err());
    tick_n_blocks(pic, 5);

    let total_supply = icrc7_total_supply(pic, controller, collection_canister_id, &());
    assert_eq!(total_supply, Nat::from(supply_cap));
}

#[test]
fn test_icrc7_transfer_chain() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let nft_owner3 = random_principal();
    let nft_owner4 = random_principal();

    // Mint a token for nft_owner1
    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    tick_n_blocks(pic, 5);

    match mint_return {
        Ok(token_id) => {
            let current_time = pic.get_time().as_nanos_since_unix_epoch();

            let transfer_args_1 = vec![icrc7::TransferArg {
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                from_subaccount: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response_1 =
                icrc7_transfer(pic, nft_owner1, collection_canister_id, &transfer_args_1);
            assert!(
                transfer_response_1[0].is_some()
                    && transfer_response_1[0].as_ref().unwrap().is_ok()
            );
            tick_n_blocks(pic, 5);

            let transfer_args_2 = vec![icrc7::TransferArg {
                to: Account {
                    owner: nft_owner3,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                from_subaccount: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response_2 =
                icrc7_transfer(pic, nft_owner2, collection_canister_id, &transfer_args_2);
            assert!(
                transfer_response_2[0].is_some()
                    && transfer_response_2[0].as_ref().unwrap().is_ok()
            );

            let transfer_args_3 = vec![icrc7::TransferArg {
                to: Account {
                    owner: nft_owner4,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                from_subaccount: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response_3 =
                icrc7_transfer(pic, nft_owner3, collection_canister_id, &transfer_args_3);
            assert!(
                transfer_response_3[0].is_some()
                    && transfer_response_3[0].as_ref().unwrap().is_ok()
            );
            tick_n_blocks(pic, 5);

            let owner_of = icrc7_owner_of(pic, controller, collection_canister_id, &vec![token_id]);
            assert_eq!(
                owner_of[0],
                Some(Account {
                    owner: nft_owner4,
                    subaccount: None
                })
            );
            tick_n_blocks(pic, 5);
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_transfer_after_fail() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(token_id) => {
            let current_time = pic.get_time().as_nanos_since_unix_epoch();

            let transfer_args_1 = vec![icrc7::TransferArg {
                to: Account {
                    owner: Principal::anonymous(),
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                from_subaccount: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response_1 =
                icrc7_transfer(pic, nft_owner1, collection_canister_id, &transfer_args_1);
            assert!(
                transfer_response_1[0].is_some()
                    && transfer_response_1[0].as_ref().unwrap().is_err()
            );

            let transfer_args_2 = vec![icrc7::TransferArg {
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                from_subaccount: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response_2 =
                icrc7_transfer(pic, nft_owner1, collection_canister_id, &transfer_args_2);
            assert!(
                transfer_response_2[0].is_some()
                    && transfer_response_2[0].as_ref().unwrap().is_ok()
            );

            let owner_of = icrc7_owner_of(pic, controller, collection_canister_id, &vec![token_id]);
            assert_eq!(
                owner_of[0],
                Some(Account {
                    owner: nft_owner2,
                    subaccount: None
                })
            );
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}
