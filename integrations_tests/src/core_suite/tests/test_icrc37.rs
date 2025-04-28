use crate::client::core_nft::{
    icrc37_approve_collection, icrc37_approve_tokens, icrc37_collection_approval_requires_token,
    icrc37_get_collection_approvals, icrc37_get_token_approvals, icrc37_is_approved,
    icrc37_max_approvals, icrc37_max_approvals_per_token_or_collection,
    icrc37_max_revoke_approvals, icrc37_revoke_collection_approvals, icrc37_revoke_token_approvals,
    icrc37_transfer_from, icrc7_owner_of,
};
use crate::core_suite::setup::default_test_setup;
use crate::core_suite::setup::setup::TestEnv;
use crate::utils::mint_nft;
use crate::utils::random_principal;

use candid::Nat;
use core_nft::types::icrc37;
use icrc_ledger_types::icrc1::account::Account;
use std::time::UNIX_EPOCH;

#[test]
fn test_icrc37_approve_tokens() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

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

            let approval_info = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            let approve_args = vec![icrc37::icrc37_approve_tokens::ApproveTokenArg {
                token_id: token_id.clone(),
                approval_info: approval_info.clone(),
            }];

            let approve_response =
                icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &approve_args);

            assert!(approve_response.is_ok());
            let results = approve_response.unwrap();
            println!("results: {:?}", results);
            assert!(results[0].is_some());
            match results[0].as_ref().unwrap() {
                icrc37::icrc37_approve_tokens::ApproveTokenResult::Ok(_) => assert!(true),
                icrc37::icrc37_approve_tokens::ApproveTokenResult::Err(_) => assert!(false),
            }

            // Verify the approval exists
            let is_approved = icrc37_is_approved(
                pic,
                controller,
                collection_canister_id,
                &vec![icrc37::icrc37_is_approved::IsApprovedArg {
                    spender: Account {
                        owner: nft_owner2,
                        subaccount: None,
                    },
                    from_subaccount: None,
                    token_id: token_id.clone(),
                }],
            );

            assert_eq!(is_approved[0], true);
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc37_approve_collection() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let current_time = pic
        .get_time()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let approval_info = icrc37::ApprovalInfo {
        spender: Account {
            owner: nft_owner2,
            subaccount: None,
        },
        from_subaccount: None,
        expires_at: None,
        memo: None,
        created_at_time: current_time,
    };

    let approve_args = vec![icrc37::icrc37_approve_collection::ApproveCollectionArg {
        approval_info: approval_info.clone(),
    }];

    let approve_response =
        icrc37_approve_collection(pic, controller, collection_canister_id, &approve_args);

    assert!(approve_response.is_ok());
    let results = approve_response.unwrap();
    assert!(results[0].is_some());
    match results[0].as_ref().unwrap() {
        icrc37::icrc37_approve_collection::ApproveCollectionResult::Ok(_) => assert!(true),
        icrc37::icrc37_approve_collection::ApproveCollectionResult::Err(_) => {
            assert!(false, "Approve collection failed");
        }
    }

    // Verify the collection approval exists
    let approvals = icrc37_get_collection_approvals(
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

    assert!(!approvals.is_empty());
    assert_eq!(approvals[0].approval_info.spender.owner, nft_owner2);
}

#[test]
fn test_icrc37_revoke_token_approvals() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    // Mint a token and approve it first
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

            // First approve the token
            let approval_info = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            let approve_args = vec![icrc37::icrc37_approve_tokens::ApproveTokenArg {
                token_id: token_id.clone(),
                approval_info: approval_info.clone(),
            }];

            let _ = icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &approve_args);

            // Now revoke the approval
            let revoke_args = vec![
                icrc37::icrc37_revoke_token_approvals::RevokeTokenApprovalArg {
                    spender: Some(Account {
                        owner: nft_owner2,
                        subaccount: None,
                    }),
                    from_subaccount: None,
                    token_id: token_id.clone(),
                    memo: None,
                    created_at_time: Some(current_time),
                },
            ];

            let revoke_response = icrc37_revoke_token_approvals(
                pic,
                nft_owner1,
                collection_canister_id,
                &revoke_args,
            );

            assert!(revoke_response.is_ok());
            let results = revoke_response.unwrap();
            assert!(results[0].is_some());
            match results[0].as_ref().unwrap() {
                icrc37::icrc37_revoke_token_approvals::RevokeTokenApprovalResponse::Ok(_) => {
                    assert!(true)
                }
                icrc37::icrc37_revoke_token_approvals::RevokeTokenApprovalResponse::Err(_) => {
                    assert!(false)
                }
            }

            // Verify the approval is gone
            let is_approved = icrc37_is_approved(
                pic,
                controller,
                collection_canister_id,
                &vec![icrc37::icrc37_is_approved::IsApprovedArg {
                    spender: Account {
                        owner: nft_owner2,
                        subaccount: None,
                    },
                    from_subaccount: None,
                    token_id: token_id.clone(),
                }],
            );

            assert_eq!(is_approved[0], false);
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc37_transfer_from() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    // Mint a token and approve it first
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

            // First approve the token
            let approval_info = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            let approve_args = vec![icrc37::icrc37_approve_tokens::ApproveTokenArg {
                token_id: token_id.clone(),
                approval_info: approval_info.clone(),
            }];

            let _ = icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &approve_args);

            // Now transfer from nft_owner1 to nft_owner2 using nft_owner2's approval
            let transfer_args = vec![icrc37::icrc37_transfer_from::TransferFromArg {
                spender_subaccount: None,
                from: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response =
                icrc37_transfer_from(pic, nft_owner2, collection_canister_id, &transfer_args);

            assert!(transfer_response.is_ok());
            let results = transfer_response.unwrap();
            assert!(results[0].is_some());
            match results[0].as_ref().unwrap() {
                icrc37::icrc37_transfer_from::TransferFromResult::Ok(_) => assert!(true),
                icrc37::icrc37_transfer_from::TransferFromResult::Err(_) => assert!(false),
            }

            // Verify the token is now owned by nft_owner2
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
fn test_icrc37_max_approvals() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let max_approvals = icrc37_max_approvals(pic, controller, collection_canister_id, &());
    assert_eq!(max_approvals, Nat::from(100u64));
}

#[test]
fn test_icrc37_max_approvals_per_token_or_collection() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let max_approvals =
        icrc37_max_approvals_per_token_or_collection(pic, controller, collection_canister_id, &());
    assert_eq!(max_approvals, Nat::from(10u64));
}

#[test]
fn test_icrc37_max_revoke_approvals() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let max_revoke_approvals =
        icrc37_max_revoke_approvals(pic, controller, collection_canister_id, &());
    assert_eq!(max_revoke_approvals, Nat::from(25u64));
}

#[test]
fn test_icrc37_collection_approval_requires_token() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let requires_token =
        icrc37_collection_approval_requires_token(pic, controller, collection_canister_id, &());
    assert_eq!(requires_token, false);
}

#[test]
fn test_icrc37_transfer_from_unauthorized_account() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let nft_owner3 = random_principal();

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

            let approval_info = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            let approve_args = vec![icrc37::icrc37_approve_tokens::ApproveTokenArg {
                token_id: token_id.clone(),
                approval_info: approval_info.clone(),
            }];

            let _ = icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &approve_args);

            // Try to transfer using nft_owner3 (unauthorized)
            let transfer_args = vec![icrc37::icrc37_transfer_from::TransferFromArg {
                spender_subaccount: None,
                from: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                to: Account {
                    owner: nft_owner3,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response =
                icrc37_transfer_from(pic, nft_owner3, collection_canister_id, &transfer_args);

            assert!(transfer_response.is_ok());
            let results = transfer_response.unwrap();
            assert!(results[0].is_some());
            match results[0].as_ref().unwrap() {
                icrc37::icrc37_transfer_from::TransferFromResult::Ok(_) => assert!(false),
                icrc37::icrc37_transfer_from::TransferFromResult::Err(_) => assert!(true),
            }

            // Verify the token is still owned by nft_owner1
            let owner_of = icrc7_owner_of(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            assert_eq!(
                owner_of[0],
                Some(Account {
                    owner: nft_owner1,
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
fn test_icrc37_transfer_from_multiple_approvals_unauthorized() {
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

    // Mint a token and approve it for multiple accounts
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

            // Approve for nft_owner2
            let approval_info_2 = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            // Approve for nft_owner3
            let approval_info_3 = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner3,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            let approve_args = vec![
                icrc37::icrc37_approve_tokens::ApproveTokenArg {
                    token_id: token_id.clone(),
                    approval_info: approval_info_2.clone(),
                },
                icrc37::icrc37_approve_tokens::ApproveTokenArg {
                    token_id: token_id.clone(),
                    approval_info: approval_info_3.clone(),
                },
            ];

            let _ = icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &approve_args);

            // Try to transfer using nft_owner4 (unauthorized)
            let transfer_args = vec![icrc37::icrc37_transfer_from::TransferFromArg {
                spender_subaccount: None,
                from: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                to: Account {
                    owner: nft_owner4,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response =
                icrc37_transfer_from(pic, nft_owner4, collection_canister_id, &transfer_args);

            assert!(transfer_response.is_ok());
            let results = transfer_response.unwrap();
            assert!(results[0].is_some());
            match results[0].as_ref().unwrap() {
                icrc37::icrc37_transfer_from::TransferFromResult::Ok(_) => assert!(false),
                icrc37::icrc37_transfer_from::TransferFromResult::Err(_) => assert!(true),
            }

            // Verify the token is still owned by nft_owner1
            let owner_of = icrc7_owner_of(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            assert_eq!(
                owner_of[0],
                Some(Account {
                    owner: nft_owner1,
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
fn test_icrc37_transfer_from_single_approval_authorized() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    // Mint a token and approve it for nft_owner2
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

            // Approve for nft_owner2
            let approval_info = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            let approve_args = vec![icrc37::icrc37_approve_tokens::ApproveTokenArg {
                token_id: token_id.clone(),
                approval_info: approval_info.clone(),
            }];

            let _ = icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &approve_args);

            // Transfer using nft_owner2 (authorized)
            let transfer_args = vec![icrc37::icrc37_transfer_from::TransferFromArg {
                spender_subaccount: None,
                from: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response =
                icrc37_transfer_from(pic, nft_owner2, collection_canister_id, &transfer_args);

            assert!(transfer_response.is_ok());
            let results = transfer_response.unwrap();
            assert!(results[0].is_some());
            match results[0].as_ref().unwrap() {
                icrc37::icrc37_transfer_from::TransferFromResult::Ok(_) => assert!(true),
                icrc37::icrc37_transfer_from::TransferFromResult::Err(_) => assert!(false),
            }

            // Verify the token is now owned by nft_owner2
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
fn test_icrc37_transfer_from_multiple_approvals_authorized() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let nft_owner3 = random_principal();

    // Mint a token and approve it for multiple accounts
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

            // Approve for nft_owner2
            let approval_info_2 = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            // Approve for nft_owner3
            let approval_info_3 = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner3,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            let approve_args = vec![
                icrc37::icrc37_approve_tokens::ApproveTokenArg {
                    token_id: token_id.clone(),
                    approval_info: approval_info_2.clone(),
                },
                icrc37::icrc37_approve_tokens::ApproveTokenArg {
                    token_id: token_id.clone(),
                    approval_info: approval_info_3.clone(),
                },
            ];

            let _ = icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &approve_args);

            // Transfer using nft_owner2 (authorized)
            let transfer_args = vec![icrc37::icrc37_transfer_from::TransferFromArg {
                spender_subaccount: None,
                from: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response =
                icrc37_transfer_from(pic, nft_owner2, collection_canister_id, &transfer_args);

            assert!(transfer_response.is_ok());
            let results = transfer_response.unwrap();
            assert!(results[0].is_some());
            match results[0].as_ref().unwrap() {
                icrc37::icrc37_transfer_from::TransferFromResult::Ok(_) => assert!(true),
                icrc37::icrc37_transfer_from::TransferFromResult::Err(_) => assert!(false),
            }

            // Verify the token is now owned by nft_owner2
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
fn test_icrc37_transfer_from_multiple_approvals_sequential_transfers() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let nft_owner3 = random_principal();

    // Mint a token and approve it for multiple accounts
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

            // Approve for nft_owner2
            let approval_info_2 = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            // Approve for nft_owner3
            let approval_info_3 = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner3,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            let approve_args = vec![
                icrc37::icrc37_approve_tokens::ApproveTokenArg {
                    token_id: token_id.clone(),
                    approval_info: approval_info_2.clone(),
                },
                icrc37::icrc37_approve_tokens::ApproveTokenArg {
                    token_id: token_id.clone(),
                    approval_info: approval_info_3.clone(),
                },
            ];

            let _ = icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &approve_args);

            // First transfer using nft_owner2 (authorized)
            let transfer_args_1 = vec![icrc37::icrc37_transfer_from::TransferFromArg {
                spender_subaccount: None,
                from: Account {
                    owner: nft_owner1,
                    subaccount: None,
                },
                to: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response_1 =
                icrc37_transfer_from(pic, nft_owner2, collection_canister_id, &transfer_args_1);

            assert!(transfer_response_1.is_ok());
            let results_1 = transfer_response_1.unwrap();
            assert!(results_1[0].is_some());
            match results_1[0].as_ref().unwrap() {
                icrc37::icrc37_transfer_from::TransferFromResult::Ok(_) => assert!(true),
                icrc37::icrc37_transfer_from::TransferFromResult::Err(_) => assert!(false),
            }

            // Verify the token is now owned by nft_owner2
            let owner_of_1 = icrc7_owner_of(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            assert_eq!(
                owner_of_1[0],
                Some(Account {
                    owner: nft_owner2,
                    subaccount: None
                })
            );

            // Try second transfer using nft_owner3 (should fail as token is now owned by nft_owner2)
            let transfer_args_2 = vec![icrc37::icrc37_transfer_from::TransferFromArg {
                spender_subaccount: None,
                from: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                to: Account {
                    owner: nft_owner3,
                    subaccount: None,
                },
                token_id: token_id.clone(),
                memo: None,
                created_at_time: Some(current_time),
            }];

            let transfer_response_2 =
                icrc37_transfer_from(pic, nft_owner3, collection_canister_id, &transfer_args_2);

            assert!(transfer_response_2.is_ok());
            let results_2 = transfer_response_2.unwrap();
            assert!(results_2[0].is_some());
            match results_2[0].as_ref().unwrap() {
                icrc37::icrc37_transfer_from::TransferFromResult::Ok(_) => assert!(false),
                icrc37::icrc37_transfer_from::TransferFromResult::Err(_) => assert!(true),
            }

            // Verify the token is still owned by nft_owner2
            let owner_of_2 = icrc7_owner_of(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            assert_eq!(
                owner_of_2[0],
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
