use crate::client::core_nft::{
    icrc7_atomic_batch_transfers, icrc7_balance_of, icrc7_default_take_value, icrc7_description,
    icrc7_logo, icrc7_max_memo_size, icrc7_max_query_batch_size, icrc7_max_take_value,
    icrc7_max_update_batch_size, icrc7_name, icrc7_owner_of, icrc7_permitted_drift,
    icrc7_supply_cap, icrc7_symbol, icrc7_token_metadata, icrc7_tokens, icrc7_tokens_of,
    icrc7_total_supply, icrc7_transfer, icrc7_tx_window, mint, update_minting_authorities,
    update_nft_metadata,
};
use crate::utils::mint_nft;
use candid::{Nat, Principal};
use core_nft::types::icrc7;
use core_nft::types::update_nft_metadata;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use icrc_ledger_types::icrc1::account::Account;
use serde_bytes::ByteBuf;
use std::collections::HashMap;

use crate::core_suite::setup::setup::TestEnv;
use crate::{core_suite::setup::default_test_setup, utils::tick_n_blocks};

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
        collection_canister_id,
    );

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
            let mut new_metadata: HashMap<String, Value> = HashMap::new();
            new_metadata.insert("test1".to_string(), Value::Text("test1".to_string()));
            new_metadata.insert("test2".to_string(), Value::Nat(Nat::from(1 as u64)));
            let logo_data = include_bytes!("../assets/logo2.min-3f9527e7.svg").to_vec();
            new_metadata.insert(
                "test3".to_string(),
                Value::Blob(ByteBuf::from(logo_data.clone())),
            );

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
                &update_nft_metadata_args,
            );

            pic.tick();

            let metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            assert_eq!(metadata[0].clone().unwrap()[0].0, "Description".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[0].1,
                Value::Text("description".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[1].0, "Logo".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[1].1,
                Value::Text("logo".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[2].0, "Name".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[2].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[3].0, "Symbol".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[3].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[4].0, "test1".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[4].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[5].0, "test2".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[5].1,
                Value::Nat(Nat::from(1 as u64))
            );
            assert_eq!(metadata[0].clone().unwrap()[6].0, "test3".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[6].1,
                Value::Blob(ByteBuf::from(logo_data))
            );
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
            let mut new_metadata: HashMap<String, Value> = HashMap::new();
            new_metadata.insert("test1".to_string(), Value::Text("test1".to_string()));
            new_metadata.insert("test2".to_string(), Value::Nat(Nat::from(1 as u64)));
            let logo_data = include_bytes!("../assets/logo2.min-3f9527e7.svg").to_vec();
            new_metadata.insert(
                "test3".to_string(),
                Value::Blob(ByteBuf::from(logo_data.clone())),
            );

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
                &update_nft_metadata_args,
            );

            pic.tick();

            let metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            assert_eq!(metadata[0].clone().unwrap()[0].0, "Description".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[0].1,
                Value::Text("description".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[1].0, "Logo".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[1].1,
                Value::Text("logo".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[2].0, "Name".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[2].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[3].0, "Symbol".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[3].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[4].0, "test1".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[4].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[5].0, "test2".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[5].1,
                Value::Nat(Nat::from(1 as u64))
            );
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
                &update_nft_metadata_args_2,
            );

            let metadata_2 = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            println!("metadata_2: {:?}", metadata_2);

            assert_eq!(metadata[0].clone().unwrap()[0].0, "Description".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[0].1,
                Value::Text("description".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[1].0, "Logo".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[1].1,
                Value::Text("logo".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[2].0, "Name".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[2].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[3].0, "Symbol".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[3].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[4].0, "test1".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[4].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[5].0, "test2".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[5].1,
                Value::Nat(Nat::from(1 as u64))
            );
            assert_eq!(metadata[0].clone().unwrap()[6].0, "test3".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[6].1,
                Value::Blob(ByteBuf::from(logo_data.clone()))
            );
            assert_eq!(metadata_2[0].clone().unwrap()[7].0, "test4".to_string());
            assert_eq!(
                metadata_2[0].clone().unwrap()[7].1,
                Value::Text("test4".to_string())
            );
            assert_eq!(metadata_2[0].clone().unwrap()[8].0, "test5".to_string());
            assert_eq!(
                metadata_2[0].clone().unwrap()[8].1,
                Value::Nat(Nat::from(2 as u64))
            );
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
                &update_nft_metadata_args,
            );

            pic.tick();

            let metadata = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            println!("metadata: {:?}", metadata);
            assert_eq!(metadata[0].clone().unwrap()[0].0, "Description".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[0].1,
                Value::Text("description".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[1].0, "Logo".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[1].1,
                Value::Text("logo".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[2].0, "Name".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[2].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[3].0, "Symbol".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[3].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[4].0, "test1".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[4].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata[0].clone().unwrap()[5].0, "test2".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[5].1,
                Value::Nat(Nat::from(1 as u64))
            );
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
                &update_nft_metadata_args_2,
            );

            let metadata_2 = icrc7_token_metadata(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            println!("metadata_2: {:?}", metadata_2);

            assert_eq!(
                metadata_2[0].clone().unwrap()[0].0,
                "Description".to_string()
            );
            assert_eq!(
                metadata_2[0].clone().unwrap()[0].1,
                Value::Text("description".to_string())
            );
            assert_eq!(metadata_2[0].clone().unwrap()[1].0, "Logo".to_string());
            assert_eq!(
                metadata_2[0].clone().unwrap()[1].1,
                Value::Text("logo".to_string())
            );
            assert_eq!(metadata_2[0].clone().unwrap()[2].0, "Name".to_string());
            assert_eq!(
                metadata_2[0].clone().unwrap()[2].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata_2[0].clone().unwrap()[3].0, "Symbol".to_string());
            assert_eq!(
                metadata_2[0].clone().unwrap()[3].1,
                Value::Text("test1".to_string())
            );
            assert_eq!(metadata_2[0].clone().unwrap()[4].0, "test1".to_string());
            assert_eq!(
                metadata_2[0].clone().unwrap()[4].1,
                Value::Text("test4".to_string())
            );
            assert_eq!(metadata_2[0].clone().unwrap()[5].0, "test2".to_string());
            assert_eq!(
                metadata_2[0].clone().unwrap()[5].1,
                Value::Nat(Nat::from(2 as u64))
            );
            assert_eq!(metadata_2[0].clone().unwrap().len(), 6);
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

    let supply_cap = icrc7_supply_cap(pic, controller, collection_canister_id, &());

    println!("supply_cap: {:?}", supply_cap);
    assert!(supply_cap == None);
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
fn test_icrc7_transfer_no_argument() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let transfer_response = icrc7_transfer(pic, controller, collection_canister_id, &vec![]);

    println!("transfer_response: {:?}", transfer_response);
    assert!(transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_err());
    assert_eq!(
        transfer_response[0]
            .as_ref()
            .unwrap()
            .as_ref()
            .err()
            .unwrap()
            .1,
        "No argument provided".to_string()
    );
}

#[test]
fn test_icrc7_transfer_exceed_max_batch_size() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let max_update_batch_size =
        icrc7_max_update_batch_size(pic, controller, collection_canister_id, &())
            .unwrap_or(Nat::from(icrc7::DEFAULT_MAX_UPDATE_BATCH_SIZE));

    let transfer_args: Vec<icrc7::TransferArg> =
        (0..u64::try_from(max_update_batch_size.0).unwrap() + 1)
            .map(|_| icrc7::TransferArg {
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: Nat::from(1 as u64),
                memo: None,
                from_subaccount: None,
                created_at_time: None,
            })
            .collect();

    let transfer_response = icrc7_transfer(pic, controller, collection_canister_id, &transfer_args);

    println!("transfer_response: {:?}", transfer_response);
    assert!(transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_err());
    assert_eq!(
        transfer_response[0]
            .as_ref()
            .unwrap()
            .as_ref()
            .err()
            .unwrap()
            .1,
        "Exceed Max allowed Update Batch Size".to_string()
    );
}

#[test]
fn test_icrc7_transfer_anonymous_identity() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let transfer_args = icrc7::TransferArg {
        to: Account {
            owner: nft_owner2,
            subaccount: None,
        },
        token_id: Nat::from(1 as u64),
        memo: None,
        from_subaccount: None,
        created_at_time: None,
    };

    let transfer_response = icrc7_transfer(
        pic,
        Principal::anonymous(),
        collection_canister_id,
        &vec![transfer_args],
    );

    println!("transfer_response: {:?}", transfer_response);
    assert!(transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_err());
    assert_eq!(
        transfer_response[0]
            .as_ref()
            .unwrap()
            .as_ref()
            .err()
            .unwrap()
            .1,
        "Anonymous Identity".to_string()
    );
}

#[test]
fn test_icrc7_transfer_non_existing_token() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let transfer_args = icrc7::TransferArg {
        to: Account {
            owner: nft_owner2,
            subaccount: None,
        },
        token_id: Nat::from(9999 as u64),
        memo: None,
        from_subaccount: None,
        created_at_time: None,
    };

    let transfer_response = icrc7_transfer(
        pic,
        controller,
        collection_canister_id,
        &vec![transfer_args],
    );

    println!("transfer_response: {:?}", transfer_response);
    assert!(transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_err());
    assert_eq!(
        transfer_response[0]
            .as_ref()
            .unwrap()
            .as_ref()
            .err()
            .unwrap()
            .1,
        "Token does not exist".to_string()
    );
}

#[test]
fn test_icrc7_transfer_invalid_memo() {
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
                memo: Some(ByteBuf::from(vec![
                    0;
                    icrc7::DEFAULT_MAX_MEMO_SIZE as usize + 1
                ])),
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
                transfer_response[0]
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .err()
                    .unwrap()
                    .1,
                "Exceeds Max Memo Size".to_string()
            );
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_transfer_unauthorized() {
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
                nft_owner2,
                collection_canister_id,
                &vec![transfer_args],
            );

            println!("transfer_response: {:?}", transfer_response);
            assert!(
                transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_err()
            );
            assert_eq!(
                transfer_response[0]
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .err()
                    .unwrap()
                    .1,
                "Token owner does not match the sender".to_string()
            );
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_transfer_to_same_owner() {
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
                    owner: nft_owner1,
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
                transfer_response[0]
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .err()
                    .unwrap()
                    .1,
                "Cannot transfer to the same owner".to_string()
            );
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc7_max_query_batch_size() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let max_query_batch_size =
        icrc7_max_query_batch_size(pic, controller, collection_canister_id, &());

    println!("max_query_batch_size: {:?}", max_query_batch_size);
    assert!(max_query_batch_size == None);
}

#[test]
fn test_icrc7_default_take_value() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let default_take_value = icrc7_default_take_value(pic, controller, collection_canister_id, &());

    println!("default_take_value: {:?}", default_take_value);
    assert!(default_take_value == None);
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

    let tokens = icrc7_tokens(pic, controller, collection_canister_id, &(None, None));
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

    let tokens = icrc7_tokens(pic, controller, collection_canister_id, &(None, None));
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

    let tokens = icrc7_tokens(pic, controller, collection_canister_id, &(None, None));
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

    let tokens_of_owner1 = icrc7_tokens_of(
        pic,
        controller,
        collection_canister_id,
        &(
            Account {
                owner: nft_owner1,
                subaccount: None,
            },
            None,
            None,
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

    let tokens_of_owner1 = icrc7_tokens_of(
        pic,
        controller,
        collection_canister_id,
        &(
            Account {
                owner: nft_owner1,
                subaccount: None,
            },
            None,
            None,
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

    let tokens_of_owner2 = icrc7_tokens_of(
        pic,
        controller,
        collection_canister_id,
        &(
            Account {
                owner: nft_owner2,
                subaccount: None,
            },
            None,
            None,
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
