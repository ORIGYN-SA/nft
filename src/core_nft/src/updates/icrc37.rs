use crate::state::{icrc3_add_transaction, mutate_state, read_state};
pub use crate::types::icrc37::{
    icrc37_approve_collection, icrc37_approve_tokens, icrc37_revoke_collection_approvals,
    icrc37_revoke_token_approvals, icrc37_transfer_from, Approval,
};
use crate::types::nft;
use crate::types::Icrc3Transaction;

use bity_ic_icrc3::types::Icrc3Error;
use bity_ic_icrc3::utils::trace;
use candid::{Nat, Principal};
use ic_cdk_macros::update;
use icrc_ledger_types::icrc1::account::Account;
use std::collections::HashMap;

async fn verify_approval_timing(
    created_at_time: u64,
    current_time: u64,
) -> Result<(), (bool, u64)> {
    let permited_drift = read_state(|state| state.data.permitted_drift.clone())
        .unwrap_or(Nat::from(crate::types::icrc7::DEFAULT_PERMITTED_DRIFT));

    if created_at_time
        > current_time
            + permited_drift
                .0
                .try_into()
                .unwrap_or(crate::types::icrc7::DEFAULT_PERMITTED_DRIFT)
    {
        return Err((true, current_time));
    }

    let tx_window = read_state(|state| {
        state
            .data
            .tx_window
            .clone()
            .unwrap_or(Nat::from(crate::types::icrc7::DEFAULT_TX_WINDOW))
    });

    if created_at_time + tx_window.0.try_into().unwrap_or(0) < current_time {
        return Err((false, 0));
    }

    Ok(())
}

#[update]
async fn icrc37_approve_tokens(
    args: icrc37_approve_tokens::Args,
) -> icrc37_approve_tokens::Response {
    let caller = ic_cdk::caller();

    let mut results = Vec::with_capacity(args.len());

    for arg in args {
        let current_time = ic_cdk::api::time(); // get current time each time because of the async calls.
        let result = approve_token(arg, caller, current_time).await;
        results.push(Some(result));
    }

    Ok(results)
}

async fn approve_token(
    arg: icrc37_approve_tokens::ApproveTokenArg,
    caller: Principal,
    current_time: u64,
) -> icrc37_approve_tokens::ApproveTokenResult {
    use icrc37_approve_tokens::{ApproveTokenError, ApproveTokenResult};

    match verify_approval_timing(arg.approval_info.created_at_time, current_time).await {
        Err((true, ledger_time)) => {
            return ApproveTokenResult::Err(ApproveTokenError::CreatedInFuture { ledger_time });
        }
        Err((false, _)) => {
            return ApproveTokenResult::Err(ApproveTokenError::TooOld);
        }
        Ok(()) => {}
    }

    let from_account = match arg.approval_info.from_subaccount {
        Some(subaccount) => Account {
            owner: caller,
            subaccount: Some(subaccount),
        },
        None => Account {
            owner: caller,
            subaccount: None,
        },
    };

    let owner = read_state(|state| state.data.owner_of(&arg.token_id));

    if owner.is_none() {
        return ApproveTokenResult::Err(ApproveTokenError::NonExistingTokenId);
    }

    if owner != Some(from_account.clone()) {
        return ApproveTokenResult::Err(ApproveTokenError::Unauthorized);
    }

    let anonymous_account = Account {
        owner: Principal::anonymous(),
        subaccount: None,
    };

    if arg.approval_info.spender == anonymous_account {
        return ApproveTokenResult::Err(ApproveTokenError::InvalidSpender);
    }

    let max_approvals_per_token = usize::try_from(
        read_state(|state| {
            state
                .data
                .approval_init
                .max_approvals_per_token_or_collection
                .clone()
        })
        .unwrap_or(Nat::from(
            crate::types::icrc37::DEFAULT_MAX_APPROVALS_PER_TOKEN_OR_COLLECTION,
        ))
        .0,
    )
    .unwrap_or(crate::types::icrc37::DEFAULT_MAX_APPROVALS_PER_TOKEN_OR_COLLECTION);

    trace(&format!(
        "max_approvals_per_token: {:?}",
        max_approvals_per_token
    ));

    let memo_clone = arg.approval_info.memo.clone();

    let approval = Approval {
        spender: arg.approval_info.spender.clone(),
        from: from_account.clone(),
        expires_at: arg.approval_info.expires_at,
        created_at: current_time,
        memo: memo_clone.clone().map(|m| m.into_vec()),
    };

    let would_exceed_max_approvals = read_state(|state| {
        let token_approvals = state
            .data
            .token_approvals
            .get(&arg.token_id)
            .cloned()
            .unwrap_or_default();

        token_approvals.len() >= max_approvals_per_token
            && !token_approvals.contains_key(&from_account)
    });

    if would_exceed_max_approvals {
        return ApproveTokenResult::Err(ApproveTokenError::GenericError {
            error_code: Nat::from(1u64),
            message: "Maximum approvals per token exceeded".to_string(),
        });
    }

    let transaction = Icrc3Transaction {
        btype: "37approve".to_string(),
        timestamp: current_time,
        tx: crate::types::transaction::TransactionData {
            tid: Some(arg.token_id.clone()),
            from: Some(from_account.clone()),
            to: None,
            meta: None,
            memo: memo_clone,
            created_at_time: Some(Nat::from(arg.approval_info.created_at_time)),
            spender: Some(arg.approval_info.spender.clone()),
            exp: None,
        },
    };

    let index = match icrc3_add_transaction(transaction).await {
        Ok(index) => index,
        Err(e) => {
            return ApproveTokenResult::Err(ApproveTokenError::GenericError {
                error_code: Nat::from(1u64),
                message: format!("Failed to log transaction: {}", e),
            });
        }
    };

    mutate_state(|state| {
        let token_approvals = state
            .data
            .token_approvals
            .entry(arg.token_id.clone())
            .or_insert_with(HashMap::new);

        token_approvals.insert(arg.approval_info.spender.clone(), approval);
    });

    ApproveTokenResult::Ok(Nat::from(index))
}

#[update]
async fn icrc37_approve_collection(
    args: icrc37_approve_collection::Args,
) -> icrc37_approve_collection::Response {
    let caller = ic_cdk::caller();

    let mut results = Vec::with_capacity(args.len());

    for arg in args {
        let current_time = ic_cdk::api::time(); // get current time each time because of the async calls.
        let result = approve_collection(arg, caller, current_time).await;
        results.push(Some(result));
    }

    Ok(results)
}

async fn approve_collection(
    arg: icrc37_approve_collection::ApproveCollectionArg,
    caller: Principal,
    current_time: u64,
) -> icrc37_approve_collection::ApproveCollectionResult {
    use icrc37_approve_collection::{ApproveCollectionError, ApproveCollectionResult};

    match verify_approval_timing(arg.approval_info.created_at_time, current_time).await {
        Err((true, ledger_time)) => {
            return ApproveCollectionResult::Err(ApproveCollectionError::CreatedInFuture {
                ledger_time,
            });
        }
        Err((false, _)) => {
            return ApproveCollectionResult::Err(ApproveCollectionError::TooOld);
        }
        Ok(()) => {}
    }

    let from_account = Account {
        owner: caller,
        subaccount: None,
    };

    let owner = read_state(|state| state.data.authorized_principals.clone());

    let anonymous_account = Account {
        owner: Principal::anonymous(),
        subaccount: None,
    };

    if arg.approval_info.spender == anonymous_account {
        return ApproveCollectionResult::Err(ApproveCollectionError::InvalidSpender);
    }

    if !owner.contains(&from_account.owner) {
        return ApproveCollectionResult::Err(ApproveCollectionError::GenericError {
            error_code: Nat::from(1u64),
            message: "Unauthorized".to_string(),
        });
    }

    let memo_clone = arg.approval_info.memo.clone();

    let approval = Approval {
        spender: arg.approval_info.spender.clone(),
        from: from_account.clone(),
        expires_at: arg.approval_info.expires_at,
        created_at: current_time,
        memo: memo_clone.clone().map(|m| m.into_vec()),
    };

    let max_approvals_per_collection = usize::try_from(
        read_state(|state| {
            state
                .data
                .approval_init
                .max_approvals_per_token_or_collection
                .clone()
        })
        .unwrap_or(Nat::from(
            crate::types::icrc37::DEFAULT_MAX_APPROVALS_PER_TOKEN_OR_COLLECTION,
        ))
        .0,
    )
    .unwrap_or(crate::types::icrc37::DEFAULT_MAX_APPROVALS_PER_TOKEN_OR_COLLECTION);

    let would_exceed_max_approvals =
        read_state(|state| state.data.collection_approvals.len() >= max_approvals_per_collection);

    if would_exceed_max_approvals {
        return ApproveCollectionResult::Err(ApproveCollectionError::GenericError {
            error_code: Nat::from(1u64),
            message: "Maximum approvals per collection exceeded".to_string(),
        });
    }

    let transaction = Icrc3Transaction {
        btype: "37approve_coll".to_string(),
        timestamp: current_time,
        tx: crate::types::transaction::TransactionData {
            tid: None,
            from: Some(from_account.clone()),
            to: None,
            meta: None,
            memo: memo_clone,
            created_at_time: Some(Nat::from(arg.approval_info.created_at_time)),
            spender: Some(arg.approval_info.spender.clone()),
            exp: None,
        },
    };

    let index = match icrc3_add_transaction(transaction).await {
        Ok(index) => index,
        Err(e) => {
            return ApproveCollectionResult::Err(ApproveCollectionError::GenericError {
                error_code: Nat::from(1u64),
                message: format!("Failed to log transaction: {}", e),
            });
        }
    };

    mutate_state(|state| {
        state
            .data
            .collection_approvals
            .insert(arg.approval_info.spender.clone(), approval);
    });

    ApproveCollectionResult::Ok(Nat::from(index))
}

#[update]
async fn icrc37_revoke_token_approvals(
    args: icrc37_revoke_token_approvals::Args,
) -> icrc37_revoke_token_approvals::Response {
    let caller = ic_cdk::caller();

    // here we check the max revoke approvals,
    // not that if spender is not provided, we will revoke all approvals for the token
    // even if the max revoke approvals is 0.
    // this is implementation choice, and is not a bug.
    // look more logical that way.

    let max_revoke_approvals = usize::try_from(
        read_state(|state| state.data.approval_init.max_revoke_approvals.clone())
            .unwrap_or(Nat::from(
                crate::types::icrc37::DEFAULT_MAX_APPROVALS_PER_TOKEN_OR_COLLECTION,
            ))
            .0,
    )
    .unwrap_or(crate::types::icrc37::DEFAULT_MAX_APPROVALS_PER_TOKEN_OR_COLLECTION);

    if args.len() > max_revoke_approvals {
        return Err(
            icrc37_revoke_token_approvals::RevokeTokenApprovalError::GenericError {
                error_code: Nat::from(1u64),
                message: "Maximum revoke approvals exceeded".to_string(),
            },
        );
    }

    let mut results = Vec::with_capacity(args.len());

    for arg in args {
        let current_time = ic_cdk::api::time();

        let result = revoke_token_approvals(arg, caller, current_time).await;
        results.push(Some(result));
    }

    Ok(results)
}

async fn revoke_token_approvals(
    arg: icrc37_revoke_token_approvals::RevokeTokenApprovalArg,
    caller: Principal,
    current_time: u64,
) -> icrc37_revoke_token_approvals::RevokeTokenApprovalResponse {
    use icrc37_revoke_token_approvals::{RevokeTokenApprovalError, RevokeTokenApprovalResponse};

    if let Some(created_at_time) = arg.created_at_time {
        match verify_approval_timing(created_at_time, current_time).await {
            Err((true, ledger_time)) => {
                return RevokeTokenApprovalResponse::Err(
                    RevokeTokenApprovalError::CreatedInFuture { ledger_time },
                );
            }
            Err((false, _)) => {
                return RevokeTokenApprovalResponse::Err(RevokeTokenApprovalError::TooOld);
            }
            Ok(()) => {}
        }
    }

    let from_account = match arg.from_subaccount {
        Some(subaccount) => Account {
            owner: caller,
            subaccount: Some(subaccount),
        },
        None => Account {
            owner: caller,
            subaccount: None,
        },
    };

    let owner = read_state(|state| state.data.owner_of(&arg.token_id));

    if owner.is_none() {
        return RevokeTokenApprovalResponse::Err(RevokeTokenApprovalError::NonExistingTokenId);
    }

    if owner != Some(from_account.clone()) {
        return RevokeTokenApprovalResponse::Err(RevokeTokenApprovalError::Unauthorized);
    }

    let mut token_approvals = read_state(|state| {
        state
            .data
            .token_approvals
            .get(&arg.token_id)
            .cloned()
            .unwrap_or_default()
    });

    if let Some(spender) = &arg.spender {
        token_approvals.remove(spender);
    } else {
        token_approvals.clear();
    };

    let created_at_time = arg.created_at_time.map(|t| Nat::from(t));

    let transaction = Icrc3Transaction {
        btype: "37revoke".to_string(),
        timestamp: current_time,
        tx: crate::types::transaction::TransactionData {
            tid: Some(arg.token_id.clone()),
            from: Some(from_account.clone()),
            to: None,
            meta: None,
            memo: arg.memo,
            created_at_time: created_at_time,
            spender: arg.spender.clone(),
            exp: None,
        },
    };

    let index = match icrc3_add_transaction(transaction).await {
        Ok(index) => index,
        Err(e) => {
            return RevokeTokenApprovalResponse::Err(RevokeTokenApprovalError::GenericError {
                error_code: Nat::from(1u64),
                message: format!("Failed to log transaction: {}", e),
            });
        }
    };

    mutate_state(|state| {
        state
            .data
            .token_approvals
            .insert(arg.token_id.clone(), token_approvals);
    });

    RevokeTokenApprovalResponse::Ok(Nat::from(index))
}

#[update]
async fn icrc37_revoke_collection_approvals(
    args: icrc37_revoke_collection_approvals::Args,
) -> icrc37_revoke_collection_approvals::Response {
    let caller = ic_cdk::caller();

    // here we check the max revoke approvals,
    // not that if spender is not provided, we will revoke all approvals for the collection
    // even if the max revoke approvals is 0.
    // this is implementation choice, and is not a bug.
    // look more logical that way.

    let max_revoke_approvals = usize::try_from(
        read_state(|state| state.data.approval_init.max_revoke_approvals.clone())
            .unwrap_or(Nat::from(
                crate::types::icrc37::DEFAULT_MAX_APPROVALS_PER_TOKEN_OR_COLLECTION,
            ))
            .0,
    )
    .unwrap_or(crate::types::icrc37::DEFAULT_MAX_APPROVALS_PER_TOKEN_OR_COLLECTION);

    if args.len() > max_revoke_approvals {
        return Err(
            icrc37_revoke_collection_approvals::RevokeCollectionApprovalError::GenericError {
                error_code: Nat::from(1u64),
                message: "Maximum revoke approvals exceeded".to_string(),
            },
        );
    }

    let mut results = Vec::with_capacity(args.len());

    for arg in args {
        let current_time = ic_cdk::api::time();
        let result = revoke_collection_approvals(arg, caller, current_time).await;
        results.push(Some(result));
    }

    Ok(results)
}

async fn revoke_collection_approvals(
    arg: icrc37_revoke_collection_approvals::RevokeCollectionApprovalArg,
    caller: Principal,
    current_time: u64,
) -> icrc37_revoke_collection_approvals::RevokeCollectionApprovalResult {
    use icrc37_revoke_collection_approvals::{
        RevokeCollectionApprovalError, RevokeCollectionApprovalResult,
    };

    if let Some(created_at_time) = arg.created_at_time {
        match verify_approval_timing(created_at_time, current_time).await {
            Err((true, ledger_time)) => {
                return RevokeCollectionApprovalResult::Err(
                    RevokeCollectionApprovalError::CreatedInFuture { ledger_time },
                );
            }
            Err((false, _)) => {
                return RevokeCollectionApprovalResult::Err(RevokeCollectionApprovalError::TooOld);
            }
            Ok(()) => {}
        }
    }

    let from_account = Account {
        owner: caller,
        subaccount: None,
    };

    let mut collection_approvals = read_state(|state| state.data.collection_approvals.clone());

    if let Some(spender) = &arg.spender {
        collection_approvals.remove(spender);
    } else {
        collection_approvals.clear();
    };

    let created_at_time = arg.created_at_time.map(|t| Nat::from(t));

    let transaction = Icrc3Transaction {
        btype: "37revoke_coll".to_string(),
        timestamp: current_time,
        tx: crate::types::transaction::TransactionData {
            tid: None,
            from: Some(from_account.clone()),
            to: None,
            meta: None,
            memo: arg.memo,
            created_at_time: created_at_time,
            spender: arg.spender.clone(),
            exp: None,
        },
    };

    let index = match icrc3_add_transaction(transaction).await {
        Ok(index) => index,
        Err(e) => {
            return RevokeCollectionApprovalResult::Err(
                RevokeCollectionApprovalError::GenericError {
                    error_code: Nat::from(1u64),
                    message: format!("Failed to log transaction: {}", e),
                },
            );
        }
    };

    mutate_state(|state| {
        state.data.collection_approvals = collection_approvals;
    });

    RevokeCollectionApprovalResult::Ok(Nat::from(index))
}

#[update]
async fn icrc37_transfer_from(args: icrc37_transfer_from::Args) -> icrc37_transfer_from::Response {
    let caller = ic_cdk::caller();

    let mut results = Vec::with_capacity(args.len());

    for arg in args {
        let current_time = ic_cdk::api::time();
        let result = transfer_from(arg, caller, current_time).await;
        results.push(Some(result));
    }

    Ok(results)
}

async fn transfer_from(
    arg: icrc37_transfer_from::TransferFromArg,
    caller: Principal,
    current_time: u64,
) -> icrc37_transfer_from::TransferFromResult {
    use icrc37_transfer_from::{TransferFromError, TransferFromResult};

    if let Some(created_at_time) = arg.created_at_time {
        match verify_approval_timing(created_at_time, current_time).await {
            Err((true, ledger_time)) => {
                return TransferFromResult::Err(TransferFromError::CreatedInFuture { ledger_time });
            }
            Err((false, _)) => {
                return TransferFromResult::Err(TransferFromError::TooOld);
            }
            Ok(()) => {}
        }
    }

    let mut nft: nft::Icrc7Token =
        match mutate_state(|state| state.data.tokens_list.get(&arg.token_id).cloned()) {
            Some(token) => token,
            None => {
                return TransferFromResult::Err(TransferFromError::NonExistingTokenId);
            }
        };

    if arg.from == arg.to {
        return TransferFromResult::Err(TransferFromError::InvalidRecipient);
    }

    let anonymous_account = Account {
        owner: Principal::anonymous(),
        subaccount: None,
    };

    if arg.to == anonymous_account {
        return TransferFromResult::Err(TransferFromError::InvalidRecipient);
    }

    let spender_account = Account {
        owner: caller,
        subaccount: arg.spender_subaccount,
    };

    let is_owner = nft.token_owner == arg.from;

    let has_token_approval = mutate_state(|state| {
        if let Some(token_approvals) = state.data.token_approvals.get(&arg.token_id) {
            if let Some(approval) = token_approvals.get(&spender_account) {
                if let Some(expires_at) = approval.expires_at {
                    state.data.token_approvals.remove(&arg.token_id);
                    return expires_at > current_time;
                }
                return true;
            }
        }
        false
    });

    let has_collection_approval = mutate_state(|state| {
        if let Some(approval) = state.data.collection_approvals.get(&spender_account) {
            if let Some(expires_at) = approval.expires_at {
                state.data.collection_approvals.remove(&spender_account);
                return expires_at > current_time;
            }
            return true;
        }
        false
    });

    let is_caller_token_holder = spender_account == arg.from;

    if !is_owner || (!is_caller_token_holder && !has_token_approval && !has_collection_approval) {
        return TransferFromResult::Err(TransferFromError::Unauthorized);
    }

    let transaction = Icrc3Transaction {
        btype: "37xfer".to_string(),
        timestamp: current_time,
        tx: crate::types::transaction::TransactionData {
            tid: Some(arg.token_id.clone()),
            from: Some(arg.from.clone()),
            to: Some(arg.to.clone()),
            meta: None,
            memo: arg.memo,
            created_at_time: arg.created_at_time.map(Nat::from),
            spender: Some(spender_account),
            exp: None,
        },
    };

    let index = match icrc3_add_transaction(transaction).await {
        Ok(index) => index,
        Err(e) => match e {
            Icrc3Error::Icrc3Error(e) => {
                if e.to_lowercase().contains("duplicate") {
                    return TransferFromResult::Err(TransferFromError::Duplicate {
                        duplicate_of: Nat::from(2u64), // value hardcoded for now. Need to update icrc3 to get the correct value
                    });
                }
                return TransferFromResult::Err(TransferFromError::GenericError {
                    error_code: Nat::from(1u64),
                    message: format!("Failed to insert transaction: {}", e),
                });
            }
            _ => {
                return TransferFromResult::Err(TransferFromError::GenericError {
                    error_code: Nat::from(1u64),
                    message: format!("Failed to insert transaction: {}", e),
                });
            }
        },
    };

    nft.transfer(arg.to.clone());

    mutate_state(|state| {
        state.data.update_token_by_id(&nft.token_id, &nft);
        state.data.token_approvals.remove(&arg.token_id);
    });

    TransferFromResult::Ok(Nat::from(index))
}
