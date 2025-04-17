use crate::client::core_nft::{
    icrc3_get_archives, icrc3_get_blocks, icrc7_owner_of, icrc7_transfer,
};
use crate::utils::mint_nft;
use candid::Nat;
use core_nft::types::icrc7;
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc3::blocks::GetBlocksRequest;

use crate::core_suite::setup::default_test_setup;
use crate::core_suite::setup::setup::TestEnv;

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
