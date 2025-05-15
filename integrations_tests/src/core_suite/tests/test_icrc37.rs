use crate::client::core_nft::{
    icrc37_approve_collection, icrc37_approve_tokens, icrc37_is_approved,
    icrc37_max_approvals_per_token_or_collection, icrc37_max_revoke_approvals,
    icrc37_revoke_collection_approvals, icrc37_revoke_token_approvals, icrc37_transfer_from,
    icrc7_owner_of,
};
use crate::core_suite::setup::default_test_setup;
use crate::core_suite::setup::setup::{TestEnv, MINUTE_IN_MS};
use crate::utils::random_principal;
use crate::utils::{mint_nft, tick_n_blocks};
use candid::{Encode, Nat};
use core_nft::types::icrc37;
use icrc_ledger_types::icrc1::account::Account;
use serde_bytes::ByteBuf;
use std::time::Duration;
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

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: controller,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(token_id) => {}
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
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
    let approvals: core_nft::types::icrc37::icrc37_get_collection_approvals::Response =
        crate::client::pocket::unwrap_response(
            pic.query_call(
                collection_canister_id,
                controller,
                "icrc37_get_collection_approvals",
                Encode!(
                    &Account {
                        owner: controller,
                        subaccount: None,
                    },
                    &(),
                    &()
                )
                .unwrap(),
            ),
        );

    println!("approvals: {:?}", approvals);

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
    assert_eq!(max_approvals, Some(Nat::from(10u64)));
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
    assert_eq!(max_revoke_approvals, Some(Nat::from(10u64)));
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

#[test]
fn test_icrc37_revoke_token_approvals_max_limit() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    // Mint a token
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
            // Get max approvals limit
            let max_approvals = icrc37_max_approvals_per_token_or_collection(
                pic,
                controller,
                collection_canister_id,
                &(),
            )
            .unwrap_or(Nat::from(10u64))
            .0
            .try_into()
            .unwrap_or(10);

            println!("max_approvals: {:?}", max_approvals);

            // First, try to approve up to the limit
            for i in 0..max_approvals {
                let current_time = pic
                    .get_time()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;

                let spender = random_principal();
                let approval_info = icrc37::ApprovalInfo {
                    spender: Account {
                        owner: spender,
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

                println!("approve_response: {:?}", approve_response);
                assert!(approve_response.is_ok());
                let approve_results = approve_response.unwrap();
                assert!(approve_results[0].is_some());
                match approve_results[0].as_ref().unwrap() {
                    icrc37::icrc37_approve_tokens::ApproveTokenResult::Ok(_) => assert!(true),
                    icrc37::icrc37_approve_tokens::ApproveTokenResult::Err(_) => assert!(false),
                }

                pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
                tick_n_blocks(pic, 10);
            }

            let current_time = pic
                .get_time()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;

            // Then try to approve one more, which should fail
            let spender = random_principal();
            let approval_info = icrc37::ApprovalInfo {
                spender: Account {
                    owner: spender,
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

            let approve_results = approve_response.unwrap();
            assert!(approve_results[0].is_some());
            match approve_results[0].as_ref().unwrap() {
                icrc37::icrc37_approve_tokens::ApproveTokenResult::Ok(_) => assert!(false),
                icrc37::icrc37_approve_tokens::ApproveTokenResult::Err(_) => assert!(true),
            }

            // Now test revoking approvals
            let max_revoke_approvals =
                icrc37_max_revoke_approvals(pic, controller, collection_canister_id, &())
                    .unwrap_or(Nat::from(10u64))
                    .0
                    .try_into()
                    .unwrap_or(10);

            // Get all current approvals
            let approvals: core_nft::types::icrc37::icrc37_get_token_approvals::Response =
                crate::client::pocket::unwrap_response(pic.query_call(
                    collection_canister_id,
                    controller,
                    "icrc37_get_token_approvals",
                    Encode!(&token_id.clone(), &(), &()).unwrap(),
                ));

            assert_eq!(approvals.len(), max_approvals);

            // Try to revoke approvals one by one
            for approval in approvals.iter().take(max_revoke_approvals) {
                let current_time = pic
                    .get_time()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;

                let revoke_args = vec![
                    icrc37::icrc37_revoke_token_approvals::RevokeTokenApprovalArg {
                        spender: Some(approval.approval_info.spender.clone()),
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
                assert_eq!(results.len(), 1);
                assert!(results[0].is_some());
                match results[0].as_ref().unwrap() {
                    icrc37::icrc37_revoke_token_approvals::RevokeTokenApprovalResponse::Ok(_) => {
                        assert!(true)
                    }
                    icrc37::icrc37_revoke_token_approvals::RevokeTokenApprovalResponse::Err(_) => {
                        assert!(false)
                    }
                }

                pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
                tick_n_blocks(pic, 10);
            }

            // Try to revoke one more approval, which should fail
            let current_time = pic
                .get_time()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;

            let remaining_approvals: core_nft::types::icrc37::icrc37_get_token_approvals::Response =
                crate::client::pocket::unwrap_response(pic.query_call(
                    collection_canister_id,
                    controller,
                    "icrc37_get_token_approvals",
                    Encode!(&token_id.clone(), &(), &()).unwrap(),
                ));

            if let Some(approval) = remaining_approvals.first() {
                let revoke_args = vec![
                    icrc37::icrc37_revoke_token_approvals::RevokeTokenApprovalArg {
                        spender: Some(approval.approval_info.spender.clone()),
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

                assert!(revoke_response.is_err());
            }
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc37_revoke_collection_approvals_max_limit() {
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

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: controller,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(_) => {}
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }

    // Get max approvals limit
    let max_approvals =
        icrc37_max_approvals_per_token_or_collection(pic, controller, collection_canister_id, &())
            .unwrap_or(Nat::from(10u64))
            .0
            .try_into()
            .unwrap_or(10);

    // First, try to approve up to the limit
    for i in 0..max_approvals {
        let current_time = pic
            .get_time()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let spender = random_principal();

        let approval_info = icrc37::ApprovalInfo {
            spender: Account {
                owner: spender,
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

        println!("approve_response: {:?}", approve_response);

        assert!(approve_response.is_ok());

        let approve_results = approve_response.unwrap();
        assert!(approve_results[0].is_some());
        match approve_results[0].as_ref().unwrap() {
            icrc37::icrc37_approve_collection::ApproveCollectionResult::Ok(_) => assert!(true),
            icrc37::icrc37_approve_collection::ApproveCollectionResult::Err(_) => assert!(false),
        }

        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
        tick_n_blocks(pic, 10);
    }

    // Then try to approve one more, which should fail
    let current_time = pic
        .get_time()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let spender = random_principal();

    let approval_info = icrc37::ApprovalInfo {
        spender: Account {
            owner: spender,
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
    println!("approve_response: {:?}", approve_response);

    let approve_results = approve_response.unwrap();
    assert!(approve_results[0].is_some());
    match approve_results[0].as_ref().unwrap() {
        icrc37::icrc37_approve_collection::ApproveCollectionResult::Ok(_) => assert!(false),
        icrc37::icrc37_approve_collection::ApproveCollectionResult::Err(_) => assert!(true),
    }
    // Now test revoking approvals
    let max_revoke_approvals =
        icrc37_max_revoke_approvals(pic, controller, collection_canister_id, &())
            .unwrap_or(Nat::from(10u64))
            .0
            .try_into()
            .unwrap_or(10);

    // Get all current approvals
    let approvals: core_nft::types::icrc37::icrc37_get_collection_approvals::Response =
        crate::client::pocket::unwrap_response(
            pic.query_call(
                collection_canister_id,
                controller,
                "icrc37_get_collection_approvals",
                Encode!(
                    &Account {
                        owner: controller,
                        subaccount: None,
                    },
                    &(),
                    &()
                )
                .unwrap(),
            ),
        );
    println!("approvals: {:?}", approvals);
    assert_eq!(approvals.len(), max_approvals);

    // Try to revoke approvals one by one
    for approval in approvals.iter().take(max_revoke_approvals) {
        let current_time = pic
            .get_time()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let revoke_args = vec![
            icrc37::icrc37_revoke_collection_approvals::RevokeCollectionApprovalArg {
                spender: Some(approval.approval_info.spender.clone()),
                from_subaccount: None,
                memo: None,
                created_at_time: Some(current_time),
            },
        ];

        let revoke_response = icrc37_revoke_collection_approvals(
            pic,
            controller,
            collection_canister_id,
            &revoke_args,
        );

        assert!(revoke_response.is_ok());
        let results = revoke_response.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_some());
        match results[0].as_ref().unwrap() {
            icrc37::icrc37_revoke_collection_approvals::RevokeCollectionApprovalResult::Ok(_) => {
                assert!(true)
            }
            icrc37::icrc37_revoke_collection_approvals::RevokeCollectionApprovalResult::Err(_) => {
                assert!(false)
            }
        }

        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
        tick_n_blocks(pic, 10);
    }

    // Try to revoke one more approval, which should fail
    let current_time = pic
        .get_time()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let remaining_approvals: core_nft::types::icrc37::icrc37_get_collection_approvals::Response =
        crate::client::pocket::unwrap_response(
            pic.query_call(
                collection_canister_id,
                controller,
                "icrc37_get_collection_approvals",
                Encode!(
                    &Account {
                        owner: controller,
                        subaccount: None,
                    },
                    &(),
                    &()
                )
                .unwrap(),
            ),
        );

    println!("remaining_approvals: {:?}", remaining_approvals);

    if let Some(approval) = remaining_approvals.first() {
        let revoke_args = vec![
            icrc37::icrc37_revoke_collection_approvals::RevokeCollectionApprovalArg {
                spender: Some(approval.approval_info.spender.clone()),
                from_subaccount: None,
                memo: None,
                created_at_time: Some(current_time),
            },
        ];

        let revoke_response = icrc37_revoke_collection_approvals(
            pic,
            controller,
            collection_canister_id,
            &revoke_args,
        );

        println!("revoke_response: {:?}", revoke_response);

        assert!(revoke_response.is_err());
    }
}

#[test]
fn test_icrc37_revoke_collection_approvals_within_limit() {
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
            owner: controller,
            subaccount: None,
        },
        controller,
        collection_canister_id,
    );

    match mint_return {
        Ok(_) => {}
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }

    // Get max revoke approvals limit
    let max_revoke_approvals =
        icrc37_max_revoke_approvals(pic, controller, collection_canister_id, &())
            .unwrap_or(Nat::from(10u64))
            .0
            .try_into()
            .unwrap_or(10);

    // Create approvals up to max_revoke_approvals
    for i in 0..max_revoke_approvals {
        let current_time = pic
            .get_time()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let spender = random_principal();
        let approval_info = icrc37::ApprovalInfo {
            spender: Account {
                owner: spender,
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

        println!("approve_response: {:?}", approve_response);
        assert!(approve_response.is_ok());
        let results = approve_response.unwrap();
        assert!(results[0].is_some());
        match results[0].as_ref().unwrap() {
            icrc37::icrc37_approve_collection::ApproveCollectionResult::Ok(_) => assert!(true),
            icrc37::icrc37_approve_collection::ApproveCollectionResult::Err(_) => assert!(false),
        }

        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
        tick_n_blocks(pic, 10);
    }

    // Get all current approvals
    let approvals: core_nft::types::icrc37::icrc37_get_collection_approvals::Response =
        crate::client::pocket::unwrap_response(
            pic.query_call(
                collection_canister_id,
                controller,
                "icrc37_get_collection_approvals",
                Encode!(
                    &Account {
                        owner: controller,
                        subaccount: None,
                    },
                    &(),
                    &()
                )
                .unwrap(),
            ),
        );

    assert_eq!(approvals.len(), max_revoke_approvals);

    // Try to revoke approvals one by one
    for approval in approvals {
        let current_time = pic
            .get_time()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let revoke_args = vec![
            icrc37::icrc37_revoke_collection_approvals::RevokeCollectionApprovalArg {
                spender: Some(approval.approval_info.spender.clone()),
                from_subaccount: None,
                memo: None,
                created_at_time: Some(current_time),
            },
        ];

        let revoke_response = icrc37_revoke_collection_approvals(
            pic,
            controller,
            collection_canister_id,
            &revoke_args,
        );

        assert!(revoke_response.is_ok());
        let results = revoke_response.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_some());
        match results[0].as_ref().unwrap() {
            icrc37::icrc37_revoke_collection_approvals::RevokeCollectionApprovalResult::Ok(_) => {
                assert!(true)
            }
            icrc37::icrc37_revoke_collection_approvals::RevokeCollectionApprovalResult::Err(_) => {
                assert!(false)
            }
        }

        pic.advance_time(Duration::from_millis(MINUTE_IN_MS));
        tick_n_blocks(pic, 10);
    }

    // Verify all approvals have been revoked
    let remaining_approvals: core_nft::types::icrc37::icrc37_get_collection_approvals::Response =
        crate::client::pocket::unwrap_response(
            pic.query_call(
                collection_canister_id,
                controller,
                "icrc37_get_collection_approvals",
                Encode!(
                    &Account {
                        owner: controller,
                        subaccount: None,
                    },
                    &(),
                    &()
                )
                .unwrap(),
            ),
        );

    assert!(remaining_approvals.is_empty());
}

#[test]
fn test_icrc37_approve_collection_with_nft() {
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

            // Approve collection for nft_owner2
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
                icrc37_approve_collection(pic, nft_owner1, collection_canister_id, &approve_args);

            assert!(approve_response.is_ok());
            let results = approve_response.unwrap();
            assert!(results[0].is_some());
            match results[0].as_ref().unwrap() {
                icrc37::icrc37_approve_collection::ApproveCollectionResult::Ok(_) => assert!(true),
                icrc37::icrc37_approve_collection::ApproveCollectionResult::Err(_) => {
                    assert!(false)
                }
            }

            // Verify the collection approval exists
            let approvals: core_nft::types::icrc37::icrc37_get_collection_approvals::Response =
                crate::client::pocket::unwrap_response(
                    pic.query_call(
                        collection_canister_id,
                        controller,
                        "icrc37_get_collection_approvals",
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

            assert!(!approvals.is_empty());
            assert_eq!(approvals[0].approval_info.spender.owner, nft_owner2);

            // Try to transfer the NFT using the collection approval
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
fn test_icrc37_approvals_reset_after_transfer_as_owner() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let nft_owner3 = random_principal();

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

            // Approve token for nft_owner2
            let token_approval_info = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            let token_approve_args = vec![icrc37::icrc37_approve_tokens::ApproveTokenArg {
                token_id: token_id.clone(),
                approval_info: token_approval_info.clone(),
            }];

            let token_approve_response =
                icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &token_approve_args);
            assert!(token_approve_response.is_ok());

            // Approve collection for nft_owner2
            let collection_approval_info = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            let collection_approve_args =
                vec![icrc37::icrc37_approve_collection::ApproveCollectionArg {
                    approval_info: collection_approval_info.clone(),
                }];

            let collection_approve_response = icrc37_approve_collection(
                pic,
                nft_owner1,
                collection_canister_id,
                &collection_approve_args,
            );
            assert!(collection_approve_response.is_ok());

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

            // Try second transfer using nft_owner2 (should succeed as nft_owner2 is now the owner)
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
                icrc37_transfer_from(pic, nft_owner2, collection_canister_id, &transfer_args_2);
            assert!(transfer_response_2.is_ok());
            let results = transfer_response_2.unwrap();
            assert!(results[0].is_some());
            match results[0].as_ref().unwrap() {
                icrc37::icrc37_transfer_from::TransferFromResult::Ok(_) => assert!(true),
                icrc37::icrc37_transfer_from::TransferFromResult::Err(_) => assert!(false),
            }

            // Verify the token is now owned by nft_owner3
            let owner_of_2 = icrc7_owner_of(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            assert_eq!(
                owner_of_2[0],
                Some(Account {
                    owner: nft_owner3,
                    subaccount: None
                })
            );

            // Verify token approvals are reset
            let token_approvals: core_nft::types::icrc37::icrc37_get_token_approvals::Response =
                crate::client::pocket::unwrap_response(pic.query_call(
                    collection_canister_id,
                    controller,
                    "icrc37_get_token_approvals",
                    Encode!(&token_id.clone(), &(), &()).unwrap(),
                ));

            assert!(token_approvals.is_empty());
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc37_approvals_reset_after_transfer_with_approvals() {
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

            // Approve token for nft_owner2
            let token_approval_info = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            let token_approve_args = vec![icrc37::icrc37_approve_tokens::ApproveTokenArg {
                token_id: token_id.clone(),
                approval_info: token_approval_info.clone(),
            }];

            let token_approve_response =
                icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &token_approve_args);
            assert!(token_approve_response.is_ok());

            // Approve collection for nft_owner2
            let collection_approval_info = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: None,
                memo: None,
                created_at_time: current_time,
            };

            let collection_approve_args =
                vec![icrc37::icrc37_approve_collection::ApproveCollectionArg {
                    approval_info: collection_approval_info.clone(),
                }];

            let collection_approve_response = icrc37_approve_collection(
                pic,
                nft_owner1,
                collection_canister_id,
                &collection_approve_args,
            );
            assert!(collection_approve_response.is_ok());

            // First transfer using nft_owner2 (authorized)
            let transfer_args_1 = vec![icrc37::icrc37_transfer_from::TransferFromArg {
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

            let transfer_response_1 =
                icrc37_transfer_from(pic, nft_owner2, collection_canister_id, &transfer_args_1);
            assert!(transfer_response_1.is_ok());

            // Verify the token is now owned by nft_owner3
            let owner_of_1 = icrc7_owner_of(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            assert_eq!(
                owner_of_1[0],
                Some(Account {
                    owner: nft_owner3,
                    subaccount: None
                })
            );

            // Try second transfer using nft_owner2 (should fail as approvals should be reset)
            let transfer_args_2 = vec![icrc37::icrc37_transfer_from::TransferFromArg {
                spender_subaccount: None,
                from: Account {
                    owner: nft_owner3,
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

            let transfer_response_2 =
                icrc37_transfer_from(pic, nft_owner2, collection_canister_id, &transfer_args_2);
            assert!(transfer_response_2.is_ok());
            let results = transfer_response_2.unwrap();
            assert!(results[0].is_some());
            match results[0].as_ref().unwrap() {
                icrc37::icrc37_transfer_from::TransferFromResult::Ok(_) => assert!(false),
                icrc37::icrc37_transfer_from::TransferFromResult::Err(_) => assert!(true),
            }

            // Verify the token is still owned by nft_owner3
            let owner_of_2 = icrc7_owner_of(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            assert_eq!(
                owner_of_2[0],
                Some(Account {
                    owner: nft_owner3,
                    subaccount: None
                })
            );

            // Verify token approvals are reset
            let token_approvals: core_nft::types::icrc37::icrc37_get_token_approvals::Response =
                crate::client::pocket::unwrap_response(pic.query_call(
                    collection_canister_id,
                    controller,
                    "icrc37_get_token_approvals",
                    Encode!(&token_id.clone(), &(), &()).unwrap(),
                ));
            assert!(token_approvals.is_empty());

            // Verify collection approvals are reset
            let collection_approvals: core_nft::types::icrc37::icrc37_get_collection_approvals::Response =
            crate::client::pocket::unwrap_response(pic.query_call(
                collection_canister_id,
                controller,
                "icrc37_get_collection_approvals",
                Encode!(
                    &Account {
                        owner: nft_owner1,
                        subaccount: None,
                    },
                    &(),
                    &()
                ).unwrap(),
            ));
            assert!(collection_approvals.is_empty());
        }
        Err(e) => {
            println!("Error minting NFT: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn test_icrc37_approve_and_revoke_before_transfer() {
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

            let approve_response =
                icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &approve_args);
            assert!(approve_response.is_ok());

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
                icrc37_transfer_from(pic, nft_owner2, collection_canister_id, &transfer_args);
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
fn test_icrc37_approve_with_expiration() {
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

            // Approve token for nft_owner2 with expiration in 1 minute
            let approval_info = icrc37::ApprovalInfo {
                spender: Account {
                    owner: nft_owner2,
                    subaccount: None,
                },
                from_subaccount: None,
                expires_at: Some(current_time + MINUTE_IN_MS as u64 * 1_000_000),
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

            // Advance time by 2 minutes
            pic.advance_time(Duration::from_millis(MINUTE_IN_MS * 2));
            tick_n_blocks(pic, 10);

            // Try to transfer using nft_owner2 (should fail as approval has expired)
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
                icrc37_transfer_from(pic, nft_owner2, collection_canister_id, &transfer_args);
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
fn test_icrc37_approve_with_subaccount() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let nft_owner3 = random_principal();
    let subaccount = Some([1u8; 32]);

    let mint_return = mint_nft(
        pic,
        "test1".to_string(),
        Account {
            owner: nft_owner1,
            subaccount,
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
                    subaccount,
                },
                from_subaccount: subaccount,
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

            let transfer_args = vec![icrc37::icrc37_transfer_from::TransferFromArg {
                spender_subaccount: subaccount,
                from: Account {
                    owner: nft_owner1,
                    subaccount,
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
                icrc37_transfer_from(pic, nft_owner2, collection_canister_id, &transfer_args);
            assert!(transfer_response.is_ok());
            let results = transfer_response.unwrap();
            assert!(results[0].is_some());
            match results[0].as_ref().unwrap() {
                icrc37::icrc37_transfer_from::TransferFromResult::Ok(_) => assert!(true),
                icrc37::icrc37_transfer_from::TransferFromResult::Err(_) => assert!(false),
            }

            let owner_of = icrc7_owner_of(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            assert_eq!(
                owner_of[0],
                Some(Account {
                    owner: nft_owner3,
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
fn test_icrc37_approve_with_memo() {
    let mut test_env: TestEnv = default_test_setup();
    let TestEnv {
        ref mut pic,
        collection_canister_id,
        controller,
        nft_owner1,
        nft_owner2,
    } = test_env;

    let nft_owner3 = random_principal();
    let memo = vec![1, 2, 3, 4];

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
                memo: Some(ByteBuf::from(memo.clone())),
                created_at_time: current_time,
            };

            let approve_args = vec![icrc37::icrc37_approve_tokens::ApproveTokenArg {
                token_id: token_id.clone(),
                approval_info: approval_info.clone(),
            }];

            let approve_response =
                icrc37_approve_tokens(pic, nft_owner1, collection_canister_id, &approve_args);
            assert!(approve_response.is_ok());

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
                memo: Some(ByteBuf::from(memo.clone())),
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

            let owner_of = icrc7_owner_of(
                pic,
                controller,
                collection_canister_id,
                &vec![token_id.clone()],
            );

            assert_eq!(
                owner_of[0],
                Some(Account {
                    owner: nft_owner3,
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
