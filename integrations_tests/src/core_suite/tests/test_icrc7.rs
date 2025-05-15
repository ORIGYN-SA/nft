use crate::client::core_nft::{
    icrc7_atomic_batch_transfers, icrc7_balance_of, icrc7_collection_metadata,
    icrc7_default_take_value, icrc7_description, icrc7_logo, icrc7_max_memo_size,
    icrc7_max_query_batch_size, icrc7_max_take_value, icrc7_max_update_batch_size, icrc7_name,
    icrc7_owner_of, icrc7_permitted_drift, icrc7_supply_cap, icrc7_symbol, icrc7_token_metadata,
    icrc7_total_supply, icrc7_transfer, icrc7_tx_window, mint, update_minting_authorities,
    update_nft_metadata,
};
use crate::core_suite::setup::setup::{TestEnv, MINUTE_IN_MS};
use crate::utils::mint_nft;
use crate::utils::random_principal;
use candid::{Encode, Nat, Principal};
use core_nft::types::icrc7;
use core_nft::types::update_nft_metadata;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use icrc_ledger_types::icrc1::account::Account;
use serde_bytes::ByteBuf;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

            println!("update_nft_metadata_args_2");

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

    let supply_cap = icrc7_supply_cap(pic, controller, collection_canister_id, &())
        .unwrap_or(Nat::from(100u64))
        .0
        .try_into()
        .unwrap_or(100u64);

    // Mint tokens up to supply cap
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
    }

    // Try to mint one more token, which should fail
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

    // Verify total supply is at cap
    let total_supply = icrc7_total_supply(pic, controller, collection_canister_id, &());
    assert_eq!(total_supply, Nat::from(supply_cap));
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
        transfer_response[0].clone().unwrap().err().unwrap(),
        icrc7::icrc7_transfer::TransferError::GenericError {
            error_code: Nat::from(0u64),
            message: "No argument provided".to_string(),
        }
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
        transfer_response[0].clone().unwrap().err().unwrap(),
        icrc7::icrc7_transfer::TransferError::GenericError {
            error_code: Nat::from(0u64),
            message: "Exceed Max allowed Update Batch Size".to_string(),
        }
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
        transfer_response[0].clone().unwrap().err().unwrap(),
        icrc7::icrc7_transfer::TransferError::InvalidRecipient
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
        transfer_response[0].clone().unwrap().err().unwrap(),
        icrc7::icrc7_transfer::TransferError::NonExistingTokenId
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
                transfer_response[0].clone().unwrap().err().unwrap(),
                icrc7::icrc7_transfer::TransferError::GenericError {
                    error_code: Nat::from(0u64),
                    message: "Exceeds Max Memo Size".to_string(),
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
            let nanos = pic
                .get_time()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
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
            let nanos = pic
                .get_time()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
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
            let nanos = pic
                .get_time()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
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
            let nanos = pic
                .get_time()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
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
    }

    let current_time = pic
        .get_time()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

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
    }

    let current_time = pic
        .get_time()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

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
        if let Ok(token_id) = mint_return {
            token_ids.push(token_id);
        }
    }

    let current_time = pic
        .get_time()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

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
            let current_time = pic
                .get_time()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;

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

    match mint_return {
        Ok(token_id) => {
            let current_time = pic
                .get_time()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;

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

            let owner_of = icrc7_owner_of(pic, controller, collection_canister_id, &vec![token_id]);
            assert_eq!(
                owner_of[0],
                Some(Account {
                    owner: nft_owner4,
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

    match mint_return {
        Ok(token_id) => {
            let current_time = pic
                .get_time()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;

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

            let mut new_metadata_2: HashMap<String, Value> = HashMap::new();
            new_metadata_2.insert("test3".to_string(), Value::Text("test3".to_string()));
            new_metadata_2.insert("test4".to_string(), Value::Nat(Nat::from(2 as u64)));

            let update_nft_metadata_args_2 = update_nft_metadata::Args {
                token_id: token_id.clone(),
                token_name: Some("test2".to_string()),
                token_description: Some("description2".to_string()),
                token_logo: Some("logo2".to_string()),
                token_metadata: Some(new_metadata_2),
            };

            let _ = update_nft_metadata(
                pic,
                controller,
                collection_canister_id,
                &update_nft_metadata_args_2,
            );
            pic.tick();

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

            let metadata =
                icrc7_token_metadata(pic, controller, collection_canister_id, &vec![token_id]);
            assert_eq!(metadata[0].clone().unwrap()[2].0, "Name".to_string());
            assert_eq!(
                metadata[0].clone().unwrap()[2].1,
                Value::Text("test2".to_string())
            );
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
            let current_time = pic
                .get_time()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;

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
