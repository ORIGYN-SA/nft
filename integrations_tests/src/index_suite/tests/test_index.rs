use crate::client::core_nft::{icrc7_transfer, update_nft_metadata};
use crate::client::indexer::get_blocks;
use crate::index_suite::setup::setup::MINUTE_IN_MS;
use crate::utils::{mint_nft, tick_n_blocks};
use candid::Nat;
use core_nft::types::icrc7;
use core_nft::types::update_nft_metadata;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Icrc3Value;
use icrc_ledger_types::icrc1::account::Account;
use std::time::Duration;

use crate::index_suite::setup::default_test_setup;
use crate::index_suite::setup::setup::TestEnv;
use index_icrc7::types::get_blocks::Args;

#[test]
fn test_icrc7_transfer_simple_index() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
        index_canister_id,
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

    tick_n_blocks(pic, 10);
    pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 10));

    let blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 10,
            filter: None,
            sort_by: None,
        },
    );
    println!("blocks: {:?}", blocks.blocks);

    assert_eq!(blocks.blocks.len(), 1);
    assert_eq!(blocks.blocks[0].id, Nat::from(0u64));
}

#[test]
fn test_get_blocks_multiple_mints() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
        index_canister_id,
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
                token_ids.push(token_id.clone());
                println!("Minted token {}: {:?}", i, token_id);
            }
            Err(e) => {
                println!("Error minting NFT {}: {:?}", i, e);
                assert!(false);
            }
        }

        tick_n_blocks(pic, 5);
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
    }

    // Get all blocks
    let blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 10,
            filter: None,
            sort_by: None,
        },
    );

    println!("blocks: {:?}", blocks.blocks);
    println!("Total blocks found: {}", blocks.blocks.len());

    assert_eq!(blocks.blocks.len(), 5, "Expected 5 mint blocks");

    for (i, block) in blocks.blocks.iter().enumerate() {
        assert_eq!(
            block.id,
            Nat::from(i as u64),
            "Block ID should be sequential"
        );
        println!("Block {}: {:?}", i, block);
    }
}

#[test]
fn test_get_blocks_mints_and_transfers() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
        index_canister_id,
    } = test_env;

    // Mint 3 NFTs
    let mut token_ids = Vec::new();
    for i in 0..3 {
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
                token_ids.push(token_id.clone());
                println!("Minted token {}: {:?}", i, token_id);
            }
            Err(e) => {
                println!("Error minting NFT {}: {:?}", i, e);
                assert!(false);
            }
        }

        tick_n_blocks(pic, 5);
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
    }

    for (i, token_id) in token_ids.iter().enumerate() {
        let transfer_args = vec![icrc7::TransferArg {
            to: Account {
                owner: nft_owner2,
                subaccount: None,
            },
            token_id: token_id.clone(),
            memo: None,
            from_subaccount: None,
            created_at_time: None,
        }];

        let transfer_response =
            icrc7_transfer(pic, nft_owner1, collection_canister_id, &transfer_args);

        assert!(
            transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok(),
            "Transfer {} failed: {:?}",
            i,
            transfer_response
        );

        tick_n_blocks(pic, 5);
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
    }

    // Get all blocks
    let blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filter: None,
            sort_by: None,
        },
    );

    println!("blocks: {:?}", blocks.blocks);
    println!("Total blocks found: {}", blocks.blocks.len());

    // Should have 3 mint operations + 3 transfer operations = 6 blocks
    assert_eq!(
        blocks.blocks.len(),
        6,
        "Expected 6 blocks (3 mints + 3 transfers)"
    );

    // Verify all blocks are present
    for (i, block) in blocks.blocks.iter().enumerate() {
        assert_eq!(
            block.id,
            Nat::from(i as u64),
            "Block ID should be sequential"
        );
        println!("Block {}: {:?}", i, block);
    }
}

#[test]
fn test_get_blocks_mints_transfers_and_updates() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
        index_canister_id,
    } = test_env;

    let mut token_ids = Vec::new();
    for i in 0..2 {
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
                token_ids.push(token_id.clone());
                println!("Minted token {}: {:?}", i, token_id);
            }
            Err(e) => {
                println!("Error minting NFT {}: {:?}", i, e);
                assert!(false);
            }
        }

        tick_n_blocks(pic, 5);
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
    }

    // Transfer first NFT
    let transfer_args = vec![icrc7::TransferArg {
        to: Account {
            owner: nft_owner2,
            subaccount: None,
        },
        token_id: token_ids[0].clone(),
        memo: None,
        from_subaccount: None,
        created_at_time: None,
    }];

    let transfer_response = icrc7_transfer(pic, nft_owner1, collection_canister_id, &transfer_args);

    assert!(
        transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok(),
        "Transfer failed: {:?}",
        transfer_response
    );

    tick_n_blocks(pic, 5);
    pic.advance_time(Duration::from_millis(MINUTE_IN_MS));

    // Update metadata for second NFT
    let update_metadata_args = update_nft_metadata::Args {
        token_id: token_ids[1].clone(),
        metadata: vec![
            (
                "name".to_string(),
                Icrc3Value::Text("Updated NFT".to_string()),
            ),
            (
                "description".to_string(),
                Icrc3Value::Text("Updated description".to_string()),
            ),
        ],
    };

    let update_result = update_nft_metadata(
        pic,
        controller,
        collection_canister_id,
        &update_metadata_args,
    );

    assert!(
        update_result.is_ok(),
        "Metadata update failed: {:?}",
        update_result
    );

    tick_n_blocks(pic, 5);
    pic.advance_time(Duration::from_millis(MINUTE_IN_MS));

    // Get all blocks
    let blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filter: None,
            sort_by: None,
        },
    );

    println!("blocks: {:?}", blocks.blocks);
    println!("Total blocks found: {}", blocks.blocks.len());

    // Should have 2 mint operations + 1 transfer + 1 update = 4 blocks
    assert_eq!(
        blocks.blocks.len(),
        4,
        "Expected 4 blocks (2 mints + 1 transfer + 1 update)"
    );

    // Verify all blocks are present
    for (i, block) in blocks.blocks.iter().enumerate() {
        assert_eq!(
            block.id,
            Nat::from(i as u64),
            "Block ID should be sequential"
        );
        println!("Block {}: {:?}", i, block);
    }
}

#[test]
fn test_get_blocks_large_sequence() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
        index_canister_id,
    } = test_env;

    // Mint 10 NFTs
    let mut token_ids = Vec::new();
    for i in 0..10 {
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
                token_ids.push(token_id.clone());
                println!("Minted token {}: {:?}", i, token_id);
            }
            Err(e) => {
                println!("Error minting NFT {}: {:?}", i, e);
                assert!(false);
            }
        }

        tick_n_blocks(pic, 5);
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS / 2));
    }

    // Transfer 5 NFTs
    for i in 0..5 {
        let transfer_args = vec![icrc7::TransferArg {
            to: Account {
                owner: nft_owner2,
                subaccount: None,
            },
            token_id: token_ids[i].clone(),
            memo: None,
            from_subaccount: None,
            created_at_time: None,
        }];

        let transfer_response =
            icrc7_transfer(pic, nft_owner1, collection_canister_id, &transfer_args);

        assert!(
            transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok(),
            "Transfer {} failed: {:?}",
            i,
            transfer_response
        );

        tick_n_blocks(pic, 3);
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS / 2));
    }

    // Update metadata for 3 NFTs
    for i in 5..8 {
        let update_metadata_args = update_nft_metadata::Args {
            token_id: token_ids[i].clone(),
            metadata: vec![
                (
                    "name".to_string(),
                    Icrc3Value::Text(format!("Updated NFT {}", i).to_string()),
                ),
                ("version".to_string(), Icrc3Value::Text("2.0".to_string())),
            ],
        };

        let update_result = update_nft_metadata(
            pic,
            controller,
            collection_canister_id,
            &update_metadata_args,
        );

        assert!(
            update_result.is_ok(),
            "Metadata update {} failed: {:?}",
            i,
            update_result
        );

        tick_n_blocks(pic, 3);
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS / 2));
    }

    // Get all blocks
    let blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 50,
            filter: None,
            sort_by: None,
        },
    );

    println!("blocks: {:?}", blocks.blocks);
    println!("Total blocks found: {}", blocks.blocks.len());

    // Should have 10 mint operations + 5 transfers + 3 updates = 18 blocks
    assert_eq!(
        blocks.blocks.len(),
        18,
        "Expected 18 blocks (10 mints + 5 transfers + 3 updates)"
    );

    // Verify all blocks are present and sequential
    for (i, block) in blocks.blocks.iter().enumerate() {
        println!("Block {}: {:?}", i, block);
        assert_eq!(
            block.id,
            Nat::from(i as u64),
            "Block ID should be sequential"
        );
    }
}

#[test]
fn test_get_blocks_pagination() {
    let mut test_env: TestEnv = default_test_setup();
    println!("test_env: {:?}", test_env);

    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
        index_canister_id,
    } = test_env;

    // Mint 8 NFTs
    let mut token_ids = Vec::new();
    for i in 0..8 {
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
                token_ids.push(token_id.clone());
                println!("Minted token {}: {:?}", i, token_id);
            }
            Err(e) => {
                println!("Error minting NFT {}: {:?}", i, e);
                assert!(false);
            }
        }

        tick_n_blocks(pic, 5);
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS / 2));
    }

    // Get blocks in pages
    let page1 = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 3,
            filter: None,
            sort_by: None,
        },
    );

    let page2 = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 3,
            length: 3,
            filter: None,
            sort_by: None,
        },
    );

    let page3 = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 6,
            length: 3,
            filter: None,
            sort_by: None,
        },
    );

    println!("Page 1: {} blocks", page1.blocks.len());
    println!("Page 2: {} blocks", page2.blocks.len());
    println!("Page 3: {} blocks", page3.blocks.len());

    // Verify pagination
    assert_eq!(page1.blocks.len(), 3, "Page 1 should have 3 blocks");
    assert_eq!(page2.blocks.len(), 3, "Page 2 should have 3 blocks");
    assert_eq!(page3.blocks.len(), 2, "Page 3 should have 2 blocks");

    // Verify block IDs are sequential within each page
    for (i, block) in page1.blocks.iter().enumerate() {
        assert_eq!(
            block.id,
            Nat::from(i as u64),
            "Page 1 block {} should have ID {}",
            i,
            i
        );
    }

    for (i, block) in page2.blocks.iter().enumerate() {
        assert_eq!(
            block.id,
            Nat::from((i + 3) as u64),
            "Page 2 block {} should have ID {}",
            i,
            i + 3
        );
    }

    for (i, block) in page3.blocks.iter().enumerate() {
        assert_eq!(
            block.id,
            Nat::from((i + 6) as u64),
            "Page 3 block {} should have ID {}",
            i,
            i + 6
        );
    }

    // Get all blocks at once to verify total count
    let all_blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filter: None,
            sort_by: None,
        },
    );

    assert_eq!(all_blocks.blocks.len(), 8, "Total blocks should be 8");
}
