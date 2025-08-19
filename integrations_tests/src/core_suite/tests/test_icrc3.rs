use crate::client::core_nft::{
    icrc3_get_archives, icrc3_get_blocks, icrc3_get_properties, icrc3_get_tip_certificate,
    icrc3_supported_block_types, icrc7_owner_of, icrc7_transfer,
};
use crate::utils::{mint_nft, tick_n_blocks};
use candid::Nat;
use core_nft::types::icrc7;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Icrc3Value;
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc3::blocks::GetBlocksRequest;
use std::time::Duration;

use crate::core_suite::setup::default_test_setup;
use crate::core_suite::setup::setup::{TestEnv, MINUTE_IN_MS};

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
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
        vec![("name".to_string(), Icrc3Value::Text("test".to_string()))],
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

    let archive_info = icrc3_get_archives(pic, controller, collection_canister_id, &());
    println!("archive_info: {:?}", archive_info);

    let blocks = icrc3_get_blocks(
        pic,
        controller,
        collection_canister_id,
        &vec![GetBlocksRequest {
            start: Nat::from(0u64),
            length: Nat::from(10u64),
        }],
    );
    println!("blocks: {:?}", blocks);
}

#[test]
fn test_icrc3_get_blocks_after_multiple_operations() {
    let mut test_env: TestEnv = default_test_setup();

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    // Mint multiple NFTs
    let mut token_ids = Vec::new();
    for i in 0..5 {
        let mint_return = mint_nft(
            pic,
            Account {
                owner: nft_owner1,
                subaccount: None,
            },
            controller,
            collection_canister_id,
            vec![(
                "name".to_string(),
                Icrc3Value::Text(format!("test{}", i).to_string()),
            )],
        );

        match mint_return {
            Ok(token_id) => {
                token_ids.push(token_id);
                tick_n_blocks(pic, 5);
            }
            Err(e) => {
                println!("Error minting NFT {}: {:?}", i, e);
                assert!(false);
            }
        }

        tick_n_blocks(pic, 5);
    }

    let blocks = icrc3_get_blocks(
        pic,
        controller,
        collection_canister_id,
        &vec![GetBlocksRequest {
            start: Nat::from(0u64),
            length: Nat::from(10u64),
        }],
    );

    assert_eq!(blocks.log_length, Nat::from(5u64));
    assert_eq!(blocks.archived_blocks.len(), 1);
    assert_eq!(blocks.archived_blocks[0].args.len(), 1);
    assert_eq!(
        blocks.archived_blocks[0].args[0],
        GetBlocksRequest {
            start: Nat::from(0u64),
            length: Nat::from(5u64),
        }
    );

    tick_n_blocks(pic, 5);

    let archived_blocks = icrc3_get_blocks(
        pic,
        controller,
        blocks.archived_blocks[0].callback.canister_id,
        &vec![GetBlocksRequest {
            start: Nat::from(0u64),
            length: Nat::from(5u64),
        }],
    );

    println!("archived_blocks: {:?}", archived_blocks);

    // First 5 blocks should be 7mint transactions
    for i in 0..5 {
        match &archived_blocks.blocks[i].block {
            icrc_ledger_types::icrc::generic_value::ICRC3Value::Map(map) => {
                assert_eq!(
                    map.get("btype"),
                    Some(&icrc_ledger_types::icrc::generic_value::ICRC3Value::Text(
                        "7mint".to_string()
                    )),
                    "Block {} is not a mint transaction",
                    i
                );
            }
            _ => panic!("Block is not a map"),
        }
    }

    // Transfer some of the NFTs
    for (i, token_id) in token_ids.iter().take(3).enumerate() {
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

        pic.advance_time(Duration::from_secs(1));
        tick_n_blocks(pic, 5);

        assert!(
            transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok(),
            "Failed to transfer NFT {}: {:?}",
            i,
            transfer_response
        );

        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
        tick_n_blocks(pic, 10);
    }

    // Verify transaction logs contain all operations
    let blocks = icrc3_get_blocks(
        pic,
        controller,
        collection_canister_id,
        &vec![GetBlocksRequest {
            start: Nat::from(0u64),
            length: Nat::from(10u64),
        }],
    );

    pic.advance_time(Duration::from_secs(1));
    tick_n_blocks(pic, 5);

    println!("blocks: {:?}", blocks);
    // Should have 5 mint operations + 3 transfer operations = 8 blocks
    assert_eq!(
        blocks.blocks.len(),
        3,
        "Expected 3 blocks, got {}",
        blocks.blocks.len()
    );

    // Next 3 blocks should be 7xfer transactions
    for i in 0..3 {
        match &blocks.blocks[i].block {
            icrc_ledger_types::icrc::generic_value::ICRC3Value::Map(map) => {
                assert_eq!(
                    map.get("btype"),
                    Some(&icrc_ledger_types::icrc::generic_value::ICRC3Value::Text(
                        "7xfer".to_string()
                    )),
                    "Block {} is not a transfer transaction",
                    i
                );
            }
            _ => panic!("Block is not a map"),
        }
    }
}

#[test]
fn test_icrc3_get_tip_certificate() {
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
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
        vec![(
            "name".to_string(),
            Icrc3Value::Text("test_cert".to_string()),
        )],
    );

    assert!(mint_return.is_ok(), "Failed to mint NFT: {:?}", mint_return);

    let certificate = icrc3_get_tip_certificate(pic, controller, collection_canister_id, &());

    assert!(
        !certificate.certificate.iter().all(|&x| x == 0),
        "Tip certificate should contain non-zero bytes"
    );
    assert!(
        !certificate.hash_tree.iter().all(|&x| x == 0),
        "Hash tree should contain non-zero bytes"
    );

    let certificate_2 = icrc3_get_tip_certificate(pic, controller, collection_canister_id, &());

    assert_eq!(
        certificate.certificate, certificate_2.certificate,
        "Certificate should be the same"
    );
    assert_eq!(
        certificate.hash_tree, certificate_2.hash_tree,
        "Hash tree should be the same"
    );

    let transfer_args = icrc7::TransferArg {
        to: Account {
            owner: nft_owner2,
            subaccount: None,
        },
        token_id: mint_return.unwrap(),
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

    assert!(
        transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok(),
        "Transfer should succeed"
    );

    let new_certificate = icrc3_get_tip_certificate(pic, controller, collection_canister_id, &());

    assert_ne!(
        certificate.certificate, new_certificate.certificate,
        "Certificate should change after transfer"
    );
    assert_ne!(
        certificate.hash_tree, new_certificate.hash_tree,
        "Hash tree should change after transfer"
    );
}

#[test]
fn test_icrc3_supported_block_types() {
    let mut test_env: TestEnv = default_test_setup();

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    // Get supported block types
    let block_types = icrc3_supported_block_types(pic, controller, collection_canister_id, &());

    // Verify all expected block types are present
    let expected_types = vec!["7mint", "7burn", "7xfer", "7update_token"];

    assert!(
        block_types.len() >= expected_types.len(),
        "Expected at least {} block types, got {}",
        expected_types.len(),
        block_types.len()
    );

    for expected_type in expected_types {
        let found = block_types.iter().any(|bt| bt.block_type == expected_type);
        assert!(
            found,
            "Expected block type '{}' not found in supported types",
            expected_type
        );
    }
}

#[test]
fn test_icrc3_get_properties() {
    let mut test_env: TestEnv = default_test_setup();

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let props = icrc3_get_properties(pic, controller, collection_canister_id, &());

    println!("props: {:?}", props);
    assert!(
        props.max_transactions_in_window > Nat::from(0u64),
        "Max transactions per request should be > 0"
    );
    assert!(
        props.max_blocks_per_response > Nat::from(0u64),
        "Max blocks per response should be > 0"
    );
}

#[test]
fn test_icrc3_block_range_validation() {
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
        Account {
            owner: nft_owner1,
            subaccount: None,
        },
        controller,
        collection_canister_id,
        vec![(
            "name".to_string(),
            Icrc3Value::Text("test_range".to_string()),
        )],
    );

    assert!(mint_return.is_ok(), "Failed to mint NFT: {:?}", mint_return);

    let valid_blocks = icrc3_get_blocks(
        pic,
        controller,
        collection_canister_id,
        &vec![GetBlocksRequest {
            start: Nat::from(0u64),
            length: Nat::from(10u64),
        }],
    );

    assert!(
        !valid_blocks.blocks.is_empty(),
        "Should return at least one block"
    );

    let out_of_bounds = icrc3_get_blocks(
        pic,
        controller,
        collection_canister_id,
        &vec![GetBlocksRequest {
            start: Nat::from(100u64),
            length: Nat::from(10u64),
        }],
    );

    assert!(
        out_of_bounds.blocks.is_empty(),
        "Should return empty blocks for out-of-bounds range"
    );
}
