use crate::client::core_nft::{icrc7_transfer, update_nft_metadata};
use crate::client::indexer::get_blocks;
use crate::index_suite::setup::setup::MINUTE_IN_MS;
use crate::utils::{mint_nft, tick_n_blocks};
use candid::Nat;
use core_nft::types::icrc7;
use core_nft::types::update_nft_metadata::Args as UpdateTokenMetadataArg;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Icrc3Value;
use icrc_ledger_types::icrc1::account::Account;
use std::time::Duration;

use crate::index_suite::setup::setup::TestEnv;
use crate::index_suite::setup::{default_test_setup, test_setup_no_limit};
use index_icrc7::{index::IndexType, types::get_blocks::Args};

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
            filters: vec![],
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

    let blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 10,
            filters: vec![],
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

    let blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![],
            sort_by: None,
        },
    );

    println!("blocks: {:?}", blocks.blocks);
    println!("Total blocks found: {}", blocks.blocks.len());

    assert_eq!(
        blocks.blocks.len(),
        6,
        "Expected 6 blocks (3 mints + 3 transfers)"
    );

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

    let update_metadata_args = UpdateTokenMetadataArg {
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

    let blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![],
            sort_by: None,
        },
    );

    println!("blocks: {:?}", blocks.blocks);
    println!("Total blocks found: {}", blocks.blocks.len());

    assert_eq!(
        blocks.blocks.len(),
        4,
        "Expected 4 blocks (2 mints + 1 transfer + 1 update)"
    );

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

    for i in 5..8 {
        let update_metadata_args = UpdateTokenMetadataArg {
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

    let blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 50,
            filters: vec![],
            sort_by: None,
        },
    );

    println!("blocks: {:?}", blocks.blocks);
    println!("Total blocks found: {}", blocks.blocks.len());

    assert_eq!(
        blocks.blocks.len(),
        18,
        "Expected 18 blocks (10 mints + 5 transfers + 3 updates)"
    );

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

    let page1 = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 3,
            filters: vec![],
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
            filters: vec![],
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
            filters: vec![],
            sort_by: None,
        },
    );

    println!("Page 1: {} blocks", page1.blocks.len());
    println!("Page 2: {} blocks", page2.blocks.len());
    println!("Page 3: {} blocks", page3.blocks.len());

    assert_eq!(page1.blocks.len(), 3, "Page 1 should have 3 blocks");
    assert_eq!(page2.blocks.len(), 3, "Page 2 should have 3 blocks");
    assert_eq!(page3.blocks.len(), 2, "Page 3 should have 2 blocks");

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

    let all_blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![],
            sort_by: None,
        },
    );

    assert_eq!(all_blocks.blocks.len(), 8, "Total blocks should be 8");
}

#[test]
fn test_get_blocks_with_real_filters() {
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
    for i in 0..3 {
        let owner = if i % 2 == 0 { nft_owner1 } else { nft_owner2 };
        let mint_return = mint_nft(
            pic,
            Account {
                owner,
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
                println!("Minted token {}: {:?} to owner {:?}", i, token_id, owner);
            }
            Err(e) => {
                println!("Error minting NFT {}: {:?}", i, e);
                assert!(false);
            }
        }

        tick_n_blocks(pic, 5);
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
    }

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

    pic.advance_time(Duration::from_millis(10 * MINUTE_IN_MS));
    tick_n_blocks(pic, 5);

    let all_blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![],
            sort_by: None,
        },
    );

    println!("All blocks: {} blocks", all_blocks.blocks.len());
    println!("All blocks: {:?}", all_blocks.blocks);
    assert_eq!(
        all_blocks.blocks.len(),
        4,
        "Should find exactly 4 blocks (3 mints + 1 transfer)"
    );

    let mint_blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![IndexType::BlockType("7mint".to_string())],
            sort_by: None,
        },
    );

    println!("Mint filtered blocks: {} blocks", mint_blocks.blocks.len());
    println!("Mint filtered blocks: {:?}", mint_blocks.blocks);
    assert_eq!(
        mint_blocks.blocks.len(),
        3,
        "Should find exactly 3 mint blocks"
    );

    let transfer_blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![IndexType::BlockType("7xfer".to_string())],
            sort_by: None,
        },
    );

    println!(
        "Transfer filtered blocks: {} blocks",
        transfer_blocks.blocks.len()
    );
    assert_eq!(
        transfer_blocks.blocks.len(),
        1,
        "Should find exactly 1 transfer block"
    );
}

#[test]
fn test_get_blocks_complex_filters_and_ordering() {
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
    let mut block_sequence = Vec::new();

    // Phase 1: Mint multiple NFTs with different owners
    println!("=== Phase 1: Minting NFTs ===");
    for i in 0..5 {
        let owner = if i % 2 == 0 { nft_owner1 } else { nft_owner2 };

        let mint_return = mint_nft(
            pic,
            Account {
                owner,
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
                block_sequence.push(("mint", owner, i as u64));
                println!("Minted token {}: {:?} for owner {:?}", i, token_id, owner);
            }
            Err(e) => {
                println!("Error minting NFT {}: {:?}", i, e);
                assert!(false);
            }
        }

        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
        tick_n_blocks(pic, 5);
    }

    // Phase 2: Transfer some NFTs between owners
    println!("=== Phase 2: Transferring NFTs ===");
    for i in 0..3 {
        let from_owner = if i % 2 == 0 { nft_owner1 } else { nft_owner2 };
        let to_owner = if from_owner == nft_owner1 {
            nft_owner2
        } else {
            nft_owner1
        };

        let transfer_args = vec![icrc7::TransferArg {
            to: Account {
                owner: to_owner,
                subaccount: None,
            },
            token_id: token_ids[i].clone(),
            memo: None,
            from_subaccount: None,
            created_at_time: None,
        }];

        let transfer_response =
            icrc7_transfer(pic, from_owner, collection_canister_id, &transfer_args);

        assert!(
            transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok(),
            "Transfer {} failed: {:?}",
            i,
            transfer_response
        );

        block_sequence.push(("transfer", from_owner, (5 + i) as u64));
        println!(
            "Transferred token {} from {:?} to {:?}",
            i, from_owner, to_owner
        );

        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
        tick_n_blocks(pic, 5);
    }

    // Phase 3: Update metadata for some tokens
    println!("=== Phase 3: Updating metadata ===");
    for i in 0..2 {
        let update_args = UpdateTokenMetadataArg {
            token_id: token_ids[i].clone(),
            metadata: vec![(
                "description".to_string(),
                Icrc3Value::Text(format!("Updated description for token {}", i).to_string()),
            )],
        };

        let update_response =
            update_nft_metadata(pic, controller, collection_canister_id, &update_args);

        assert!(
            update_response.is_ok(),
            "Update {} failed: {:?}",
            i,
            update_response
        );

        block_sequence.push(("update", controller, (8 + i) as u64));
        println!("Updated metadata for token {}", i);

        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
        tick_n_blocks(pic, 5);
    }

    // Phase 4: Transfer one more token to create more variety
    println!("=== Phase 4: Transferring one more token ===");
    let transfer_args = vec![icrc7::TransferArg {
        to: Account {
            owner: nft_owner2,
            subaccount: None,
        },
        token_id: token_ids[4].clone(),
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

    block_sequence.push(("transfer", nft_owner2, 10));
    println!("Transferred token 4 from nft_owner2 to nft_owner1");

    pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
    tick_n_blocks(pic, 5);

    // Wait for indexing to complete
    tick_n_blocks(pic, 10);
    pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 2));

    println!("=== Block sequence created: {:?} ===", block_sequence);
    println!("Total expected blocks: {}", block_sequence.len());

    // Test 1: Get all blocks without filters, ascending order
    println!("=== Test 1: All blocks, ascending order ===");
    let blocks_asc = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![],
            sort_by: Some(index_icrc7::index::SortBy::Ascending),
        },
    );

    println!("Blocks ascending: {:?}", blocks_asc.blocks);
    assert_eq!(
        blocks_asc.blocks.len(),
        11,
        "Expected 11 blocks (5 mints + 3 transfers + 2 updates + 1 burn)"
    );

    // Verify ascending order
    for i in 0..blocks_asc.blocks.len() - 1 {
        assert!(
            blocks_asc.blocks[i].id < blocks_asc.blocks[i + 1].id,
            "Blocks should be in ascending order"
        );
    }

    // Test 2: Get all blocks without filters, descending order
    println!("=== Test 2: All blocks, descending order ===");
    let blocks_desc = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![],
            sort_by: Some(index_icrc7::index::SortBy::Descending),
        },
    );

    println!("Blocks descending: {:?}", blocks_desc.blocks);
    assert_eq!(blocks_desc.blocks.len(), 11);

    // Verify descending order
    for i in 0..blocks_desc.blocks.len() - 1 {
        assert!(
            blocks_desc.blocks[i].id > blocks_desc.blocks[i + 1].id,
            "Blocks should be in descending order"
        );
    }

    // Test 3: Filter by account (nft_owner1), ascending order
    println!("=== Test 3: Filter by nft_owner1, ascending order ===");
    let blocks_owner1_asc = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![index_icrc7::index::IndexType::Account(
                index_icrc7::wrapped_values::WrappedAccount(Account {
                    owner: nft_owner1,
                    subaccount: None,
                }),
            )],
            sort_by: Some(index_icrc7::index::SortBy::Ascending),
        },
    );

    println!(
        "Blocks for nft_owner1 (asc): {:?}",
        blocks_owner1_asc.blocks
    );
    assert!(
        blocks_owner1_asc.blocks.len() > 0,
        "Should have blocks for nft_owner1"
    );

    // Verify ascending order for filtered results
    for i in 0..blocks_owner1_asc.blocks.len() - 1 {
        assert!(
            blocks_owner1_asc.blocks[i].id < blocks_owner1_asc.blocks[i + 1].id,
            "Filtered blocks should be in ascending order"
        );
    }

    // Test 4: Filter by account (nft_owner2), descending order
    println!("=== Test 4: Filter by nft_owner2, descending order ===");
    let blocks_owner2_desc = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![index_icrc7::index::IndexType::Account(
                index_icrc7::wrapped_values::WrappedAccount(Account {
                    owner: nft_owner2,
                    subaccount: None,
                }),
            )],
            sort_by: Some(index_icrc7::index::SortBy::Descending),
        },
    );

    println!(
        "Blocks for nft_owner2 (desc): {:?}",
        blocks_owner2_desc.blocks
    );
    assert!(
        blocks_owner2_desc.blocks.len() > 0,
        "Should have blocks for nft_owner2"
    );

    // Verify descending order for filtered results
    for i in 0..blocks_owner2_desc.blocks.len() - 1 {
        assert!(
            blocks_owner2_desc.blocks[i].id > blocks_owner2_desc.blocks[i + 1].id,
            "Filtered blocks should be in descending order"
        );
    }

    // Test 5: Filter by block type (mint), ascending order
    println!("=== Test 5: Filter by mint block type, ascending order ===");
    let blocks_mint_asc = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![index_icrc7::index::IndexType::BlockType(
                "7mint".to_string(),
            )],
            sort_by: Some(index_icrc7::index::SortBy::Ascending),
        },
    );

    println!("Mint blocks (asc): {:?}", blocks_mint_asc.blocks);
    assert_eq!(blocks_mint_asc.blocks.len(), 5, "Should have 5 mint blocks");

    // Verify ascending order for mint blocks
    for i in 0..blocks_mint_asc.blocks.len() - 1 {
        assert!(
            blocks_mint_asc.blocks[i].id < blocks_mint_asc.blocks[i + 1].id,
            "Mint blocks should be in ascending order"
        );
    }

    // Test 6: Filter by block type (transfer), descending order
    println!("=== Test 6: Filter by transfer block type, descending order ===");
    let blocks_transfer_desc = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![index_icrc7::index::IndexType::BlockType(
                "7xfer".to_string(),
            )],
            sort_by: Some(index_icrc7::index::SortBy::Descending),
        },
    );

    println!("Transfer blocks (desc): {:?}", blocks_transfer_desc.blocks);
    assert_eq!(
        blocks_transfer_desc.blocks.len(),
        4,
        "Should have 4 transfer blocks"
    );

    // Verify descending order for transfer blocks
    for i in 0..blocks_transfer_desc.blocks.len() - 1 {
        assert!(
            blocks_transfer_desc.blocks[i].id > blocks_transfer_desc.blocks[i + 1].id,
            "Transfer blocks should be in descending order"
        );
    }

    // Test 7: Combined filters (account + block type), ascending order
    println!("=== Test 7: Combined filters (nft_owner1 + mint), ascending order ===");
    let blocks_combined_asc = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![
                index_icrc7::index::IndexType::Account(
                    index_icrc7::wrapped_values::WrappedAccount(Account {
                        owner: nft_owner1,
                        subaccount: None,
                    }),
                ),
                index_icrc7::index::IndexType::BlockType("7mint".to_string()),
            ],
            sort_by: Some(index_icrc7::index::SortBy::Ascending),
        },
    );

    println!(
        "Combined filter blocks (asc): {:?}",
        blocks_combined_asc.blocks
    );
    assert!(
        blocks_combined_asc.blocks.len() > 0,
        "Should have blocks matching combined filters"
    );

    // Verify ascending order for combined filter results
    for i in 0..blocks_combined_asc.blocks.len() - 1 {
        assert!(
            blocks_combined_asc.blocks[i].id < blocks_combined_asc.blocks[i + 1].id,
            "Combined filter blocks should be in ascending order"
        );
    }

    // Test 8: Pagination with filters and ordering
    println!("=== Test 8: Pagination with filters and ordering ===");
    let blocks_paginated = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 1,  // Start from 2nd block
            length: 1, // Get 1 blocks
            filters: vec![
                index_icrc7::index::IndexType::Account(
                    index_icrc7::wrapped_values::WrappedAccount(Account {
                        owner: nft_owner1,
                        subaccount: None,
                    }),
                ),
                index_icrc7::index::IndexType::BlockType("7mint".to_string()),
            ],
            sort_by: Some(index_icrc7::index::SortBy::Ascending),
        },
    );

    println!("Paginated blocks: {:?}", blocks_paginated.blocks);
    assert_eq!(
        blocks_paginated.blocks.len(),
        1,
        "Should have 1 paginated blocks"
    );

    // Verify pagination starts from correct position
    assert_eq!(
        blocks_paginated.blocks[0].id,
        Nat::from(2u64),
        "First paginated block should have ID 2"
    );

    println!("=== All complex filter and ordering tests passed! ===");
}

#[test]
fn test_get_blocks_descending_order_with_limit() {
    let mut test_env: TestEnv = test_setup_no_limit();
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

    // Phase 1: Mint 15 NFTs to ensure we have more than 10 transactions
    println!("=== Phase 1: Minting 15 NFTs ===");
    for i in 0..15 {
        let owner = if i % 2 == 0 { nft_owner1 } else { nft_owner2 };

        let mint_return = mint_nft(
            pic,
            Account {
                owner,
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
                println!("Minted token {}: {:?} for owner {:?}", i, token_id, owner);
            }
            Err(e) => {
                println!("Error minting NFT {}: {:?}", i, e);
                assert!(false);
            }
        }

        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
        tick_n_blocks(pic, 5);
    }

    // Phase 2: Transfer some NFTs to create more transactions
    println!("=== Phase 2: Transferring some NFTs ===");
    for i in 0..5 {
        let from_owner = if i % 2 == 0 { nft_owner1 } else { nft_owner2 };
        let to_owner = if from_owner == nft_owner1 {
            nft_owner2
        } else {
            nft_owner1
        };

        let transfer_args = vec![icrc7::TransferArg {
            to: Account {
                owner: to_owner,
                subaccount: None,
            },
            token_id: token_ids[i].clone(),
            memo: None,
            from_subaccount: None,
            created_at_time: None,
        }];

        let transfer_response =
            icrc7_transfer(pic, from_owner, collection_canister_id, &transfer_args);

        assert!(
            transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok(),
            "Transfer {} failed: {:?}",
            i,
            transfer_response
        );

        println!(
            "Transferred token {} from {:?} to {:?}",
            i, from_owner, to_owner
        );

        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
        tick_n_blocks(pic, 5);
    }

    // Wait for indexing to complete
    tick_n_blocks(pic, 10);
    pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 2));

    // Total expected: 15 mints + 5 transfers = 20 transactions
    let total_expected_transactions = 20;
    println!(
        "Total expected transactions: {}",
        total_expected_transactions
    );

    // Test 1: Get all blocks in descending order to verify total count
    println!("=== Test 1: All blocks in descending order ===");
    let all_blocks_desc = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 50, // Large enough to get all blocks
            filters: vec![],
            sort_by: Some(index_icrc7::index::SortBy::Descending),
        },
    );

    println!("All blocks (desc): {:?}", all_blocks_desc.blocks);

    println!("All blocks (desc): {} blocks", all_blocks_desc.blocks.len());
    assert_eq!(
        all_blocks_desc.blocks.len(),
        total_expected_transactions,
        "Expected {} total transactions",
        total_expected_transactions
    );

    // Verify descending order
    for i in 0..all_blocks_desc.blocks.len() - 1 {
        assert!(
            all_blocks_desc.blocks[i].id > all_blocks_desc.blocks[i + 1].id,
            "Blocks should be in descending order"
        );
    }

    // Test 2: Get only 10 blocks in descending order (the main test)
    println!("=== Test 2: Descending order with limit of 10 ===");
    let blocks_desc_limit_10 = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 10,
            filters: vec![],
            sort_by: Some(index_icrc7::index::SortBy::Descending),
        },
    );

    println!(
        "Blocks descending (limit 10): {:?}",
        blocks_desc_limit_10.blocks
    );
    assert_eq!(
        blocks_desc_limit_10.blocks.len(),
        10,
        "Should have exactly 10 blocks"
    );

    // Verify that we start from the highest block ID (most recent transaction)
    let expected_first_block_id = total_expected_transactions - 1; // Since IDs start from 0
    assert_eq!(
        blocks_desc_limit_10.blocks[0].id,
        Nat::from(expected_first_block_id),
        "First block should have the highest ID (most recent transaction)"
    );

    // Verify that we end with the 10th most recent transaction
    let expected_last_block_id = total_expected_transactions - 10; // 10th most recent
    assert_eq!(
        blocks_desc_limit_10.blocks[9].id,
        Nat::from(expected_last_block_id),
        "Last block should have ID {} (10th most recent transaction)",
        expected_last_block_id
    );

    // Verify descending order within the limited results
    for i in 0..blocks_desc_limit_10.blocks.len() - 1 {
        assert!(
            blocks_desc_limit_10.blocks[i].id > blocks_desc_limit_10.blocks[i + 1].id,
            "Limited blocks should maintain descending order"
        );
    }

    // Test 3: Verify that the next 10 blocks continue the sequence correctly
    println!("=== Test 3: Next 10 blocks in descending order ===");
    let blocks_desc_next_10 = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 10,
            length: 10,
            filters: vec![],
            sort_by: Some(index_icrc7::index::SortBy::Descending),
        },
    );

    println!("Next 10 blocks (desc): {:?}", blocks_desc_next_10.blocks);
    assert_eq!(
        blocks_desc_next_10.blocks.len(),
        10,
        "Should have exactly 10 more blocks"
    );

    // Verify that the first block of the next 10 continues from where the previous 10 left off
    let expected_continuation_id = expected_last_block_id - 1 + 10;
    assert_eq!(
        blocks_desc_next_10.blocks[0].id,
        Nat::from(expected_continuation_id),
        "First block of next 10 should continue the descending sequence"
    );

    // Verify descending order within the next 10 results
    for i in 0..blocks_desc_next_10.blocks.len() - 1 {
        assert!(
            blocks_desc_next_10.blocks[i].id > blocks_desc_next_10.blocks[i + 1].id,
            "Next 10 blocks should maintain descending order"
        );
    }

    println!("=== Descending order with limit test passed! ===");
    println!("Successfully verified that descending order with limit 10 starts from the end of transactions");
}

#[test]
fn test_get_blocks_filter_by_token_id() {
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

    // Phase 1: Mint multiple NFTs to different owners
    println!("=== Phase 1: Minting NFTs ===");
    for i in 0..5 {
        let owner = if i % 2 == 0 { nft_owner1 } else { nft_owner2 };
        let mint_return = mint_nft(
            pic,
            Account {
                owner,
                subaccount: None,
            },
            controller,
            collection_canister_id,
            vec![(
                "name".to_string(),
                Icrc3Value::Text(format!("NFT Token {}", i).to_string()),
            )],
        );

        match mint_return {
            Ok(token_id) => {
                token_ids.push(token_id.clone());
                println!("Minted token {}: {:?} for owner {:?}", i, token_id, owner);
            }
            Err(e) => {
                println!("Error minting NFT {}: {:?}", i, e);
                assert!(false);
            }
        }

        tick_n_blocks(pic, 5);
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
    }

    // Phase 2: Transfer some NFTs to create more transactions for the same token IDs
    println!("=== Phase 2: Transferring NFTs ===");
    for i in 0..3 {
        let from_owner = if i % 2 == 0 { nft_owner1 } else { nft_owner2 };
        let to_owner = if from_owner == nft_owner1 {
            nft_owner2
        } else {
            nft_owner1
        };

        let transfer_args = vec![icrc7::TransferArg {
            to: Account {
                owner: to_owner,
                subaccount: None,
            },
            token_id: token_ids[i].clone(),
            memo: None,
            from_subaccount: None,
            created_at_time: None,
        }];

        let transfer_response =
            icrc7_transfer(pic, from_owner, collection_canister_id, &transfer_args);

        assert!(
            transfer_response[0].is_some() && transfer_response[0].as_ref().unwrap().is_ok(),
            "Transfer {} failed: {:?}",
            i,
            transfer_response
        );

        println!(
            "Transferred token {} from {:?} to {:?}",
            i, from_owner, to_owner
        );

        tick_n_blocks(pic, 5);
        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
    }

    // Wait for indexing to complete
    tick_n_blocks(pic, 10);
    pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 2));

    // Test 1: Get all blocks without filters to verify total count
    println!("=== Test 1: All blocks without filters ===");
    let all_blocks = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![],
            sort_by: None,
        },
    );

    println!("All blocks: {:?}", all_blocks.blocks);
    println!("Total blocks found: {}", all_blocks.blocks.len());

    // Should have 5 mints + 3 transfers = 8 blocks
    assert_eq!(
        all_blocks.blocks.len(),
        8,
        "Expected 8 blocks (5 mints + 3 transfers)"
    );

    // Test 2: Filter by specific token ID (first token)
    println!("=== Test 2: Filter by first token ID ===");
    let token_0_filter = index_icrc7::index::IndexType::TokenId(
        index_icrc7::wrapped_values::WrappedNat(token_ids[0].clone()),
    );

    let blocks_token_0 = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![token_0_filter],
            sort_by: Some(index_icrc7::index::SortBy::Ascending),
        },
    );

    println!("Blocks for token 0: {:?}", blocks_token_0.blocks);
    println!("Blocks for token 0 count: {}", blocks_token_0.blocks.len());

    // Should have 2 blocks: 1 mint + 1 transfer for token 0
    assert_eq!(
        blocks_token_0.blocks.len(),
        2,
        "Expected 2 blocks for token 0 (1 mint + 1 transfer)"
    );

    // Verify that all blocks are related to token 0
    for block in &blocks_token_0.blocks {
        println!("Block for token 0: {:?}", block);
        // Verify that the block contains the correct token ID in its transaction data
        if let icrc_ledger_types::icrc::generic_value::ICRC3Value::Map(tx_map) = &block.block {
            if let Some(icrc_ledger_types::icrc::generic_value::ICRC3Value::Map(inner_tx)) =
                tx_map.get("tx")
            {
                if let Some(icrc_ledger_types::icrc::generic_value::ICRC3Value::Nat(tid)) =
                    inner_tx.get("tid")
                {
                    assert_eq!(
                        *tid, token_ids[0],
                        "Block should contain the correct token ID"
                    );
                }
            }
        }
    }

    // Test 3: Filter by another token ID (second token)
    println!("=== Test 3: Filter by second token ID ===");
    let token_1_filter = index_icrc7::index::IndexType::TokenId(
        index_icrc7::wrapped_values::WrappedNat(token_ids[1].clone()),
    );

    let blocks_token_1 = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![token_1_filter],
            sort_by: Some(index_icrc7::index::SortBy::Descending),
        },
    );

    println!("Blocks for token 1: {:?}", blocks_token_1.blocks);
    println!("Blocks for token 1 count: {}", blocks_token_1.blocks.len());

    // Should have 2 blocks: 1 mint + 1 transfer for token 1
    assert_eq!(
        blocks_token_1.blocks.len(),
        2,
        "Expected 2 blocks for token 1 (1 mint + 1 transfer)"
    );

    // Test 4: Filter by token ID that only has mint (no transfers)
    println!("=== Test 4: Filter by token ID with only mint ===");
    let token_4_filter = index_icrc7::index::IndexType::TokenId(
        index_icrc7::wrapped_values::WrappedNat(token_ids[4].clone()),
    );

    let blocks_token_4 = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![token_4_filter],
            sort_by: None,
        },
    );

    println!("Blocks for token 4: {:?}", blocks_token_4.blocks);
    println!("Blocks for token 4 count: {}", blocks_token_4.blocks.len());

    // Should have only 1 block: mint only (no transfer for token 4)
    assert_eq!(
        blocks_token_4.blocks.len(),
        1,
        "Expected 1 block for token 4 (mint only)"
    );

    // Test 5: Filter by non-existent token ID
    println!("=== Test 5: Filter by non-existent token ID ===");
    let non_existent_token_filter = index_icrc7::index::IndexType::TokenId(
        index_icrc7::wrapped_values::WrappedNat(Nat::from(999u64)),
    );

    let blocks_non_existent = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: vec![non_existent_token_filter],
            sort_by: None,
        },
    );

    println!(
        "Blocks for non-existent token: {:?}",
        blocks_non_existent.blocks
    );
    println!(
        "Blocks for non-existent token count: {}",
        blocks_non_existent.blocks.len()
    );

    // Should have 0 blocks for non-existent token
    assert_eq!(
        blocks_non_existent.blocks.len(),
        0,
        "Expected 0 blocks for non-existent token"
    );

    // Test 6: Combined filters - token ID + block type
    println!("=== Test 6: Combined filters - token ID + block type ===");
    let combined_filters = vec![
        index_icrc7::index::IndexType::TokenId(index_icrc7::wrapped_values::WrappedNat(
            token_ids[0].clone(),
        )),
        index_icrc7::index::IndexType::BlockType("7mint".to_string()),
    ];

    let blocks_combined = get_blocks(
        pic,
        controller,
        index_canister_id,
        &Args {
            start: 0,
            length: 20,
            filters: combined_filters,
            sort_by: None,
        },
    );

    println!("Blocks for combined filter: {:?}", blocks_combined.blocks);
    println!(
        "Blocks for combined filter count: {}",
        blocks_combined.blocks.len()
    );

    // Should have only 1 block: mint for token 0 only
    assert_eq!(
        blocks_combined.blocks.len(),
        1,
        "Expected 1 block for token 0 mint only"
    );

    // Verify that the block is indeed a mint block for token 0
    let combined_block = &blocks_combined.blocks[0];
    if let icrc_ledger_types::icrc::generic_value::ICRC3Value::Map(tx_map) = &combined_block.block {
        // Check block type
        if let Some(icrc_ledger_types::icrc::generic_value::ICRC3Value::Text(btype)) =
            tx_map.get("btype")
        {
            assert_eq!(*btype, "7mint", "Block should be a mint block");
        }

        // Check token ID in transaction
        if let Some(icrc_ledger_types::icrc::generic_value::ICRC3Value::Map(inner_tx)) =
            tx_map.get("tx")
        {
            if let Some(icrc_ledger_types::icrc::generic_value::ICRC3Value::Nat(tid)) =
                inner_tx.get("tid")
            {
                assert_eq!(
                    *tid, token_ids[0],
                    "Block should contain the correct token ID"
                );
            }
        }
    }

    println!("=== NFT Token ID filter tests passed! ===");
    println!("Successfully verified filtering by NFT token ID with various scenarios");
}
