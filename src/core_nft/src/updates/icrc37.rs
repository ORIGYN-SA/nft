use crate::state::{icrc3_add_transaction, mutate_state, read_state};
pub use crate::types::icrc37::{
    icrc37_approve_collection, icrc37_approve_tokens, icrc37_revoke_collection_approvals,
    icrc37_revoke_token_approvals, icrc37_transfer_from, Approval,
};
use crate::types::nft;
use crate::types::wrapped_types::{WrappedAccount, WrappedNat, WrappedApprovalValue};
use crate::types::{__COLLECTION_APPROVALS, __TOKEN_APPROVALS};

use bity_ic_icrc3::{
    transaction::{ICRC37Transaction, ICRC37TransactionData},
    types::Icrc3Error,
};
use candid::{Nat, Principal};
use ic_cdk_macros::update;
use icrc_ledger_types::icrc1::account::Account;
use std::collections::HashMap;

use crate::guards::guard_sliding_window;

fn verify_approval_timing(created_at_time: u64, current_time: u64) -> Result<(), (bool, u64)> {
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
fn icrc37_approve_tokens(args: icrc37_approve_tokens::Args) -> icrc37_approve_tokens::Response {
    let caller = ic_cdk::api::msg_caller();

    let mut results = Vec::with_capacity(args.len());

    for arg in args {
        let current_time = ic_cdk::api::time(); // get current time each time because of the async calls.
        let result = approve_token(arg, caller, current_time);
        results.push(Some(result));
    }

    Ok(results)
}

fn approve_token(
    arg: icrc37_approve_tokens::ApproveTokenArg,
    caller: Principal,
    current_time: u64,
) -> icrc37_approve_tokens::ApproveTokenResult {
    use icrc37_approve_tokens::{ApproveTokenError, ApproveTokenResult};

    match guard_sliding_window(arg.token_id.clone()) {
        Ok(()) => {}
        Err(e) => {
            return ApproveTokenResult::Err(ApproveTokenError::GenericError {
                error_code: Nat::from(0u64),
                message: e,
            });
        }
    }

    match verify_approval_timing(arg.approval_info.created_at_time, current_time) {
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

    let memo_clone = arg.approval_info.memo.clone();

    let approval = Approval {
        spender: WrappedAccount::from(arg.approval_info.spender.clone()),
        from: WrappedAccount::from(from_account.clone()),
        expires_at: arg.approval_info.expires_at,
        created_at: current_time,
        memo: memo_clone.clone().map(|m| m.to_vec()),
    };

    let would_exceed_max_approvals = __TOKEN_APPROVALS.with_borrow(|token_approvals| {
        token_approvals
            .get(&WrappedNat::from(arg.token_id.clone()))
            .is_some()
            && token_approvals
                .get(&WrappedNat::from(arg.token_id.clone()))
                .unwrap()
                .0
                .len()
                >= max_approvals_per_token
    });

    if would_exceed_max_approvals {
        return ApproveTokenResult::Err(ApproveTokenError::GenericError {
            error_code: Nat::from(1u64),
            message: "Maximum approvals per token exceeded".to_string(),
        });
    }

    let transaction = ICRC37Transaction::new(
        "37approve".to_string(),
        current_time,
        ICRC37TransactionData {
            op: "37approve".to_string(),
            tid: Some(arg.token_id.clone()),
            from: Some(from_account.clone()),
            to: None,
            memo: memo_clone.clone(),
            created_at_time: Some(Nat::from(arg.approval_info.created_at_time)),
            spender: Some(arg.approval_info.spender),
            exp: arg.approval_info.expires_at.map(|e| Nat::from(e)),
        },
    );

    let index = match icrc3_add_transaction(transaction) {
        Ok(index) => index,
        Err(e) => {
            return ApproveTokenResult::Err(ApproveTokenError::GenericError {
                error_code: Nat::from(1u64),
                message: format!("Failed to log transaction: {}", e),
            });
        }
    };

    __TOKEN_APPROVALS.with_borrow_mut(|s| {
        let token_approvals = s.get(&WrappedNat::from(arg.token_id.clone()));

        let mut approval_map = if token_approvals.is_none() {
            HashMap::new()
        } else {
            token_approvals.unwrap().0
        };

        approval_map.insert(
            WrappedAccount::from(arg.approval_info.spender.clone()),
            approval,
        );

        s.insert(
            WrappedNat::from(arg.token_id.clone()),
            WrappedApprovalValue(approval_map),
        );
    });

    ApproveTokenResult::Ok(Nat::from(index))
}

#[update]
fn icrc37_approve_collection(
    args: icrc37_approve_collection::Args,
) -> icrc37_approve_collection::Response {
    let caller = ic_cdk::api::msg_caller();

    let mut results = Vec::with_capacity(args.len());

    for arg in args {
        let current_time = ic_cdk::api::time(); // get current time each time because of the async calls.
        let result = approve_collection(arg, caller, current_time);
        results.push(Some(result));
    }

    Ok(results)
}

fn approve_collection(
    arg: icrc37_approve_collection::ApproveCollectionArg,
    caller: Principal,
    current_time: u64,
) -> icrc37_approve_collection::ApproveCollectionResult {
    use icrc37_approve_collection::{ApproveCollectionError, ApproveCollectionResult};

    match guard_sliding_window(candid::Nat::from(0u64)) {
        // we consider the collection as a token with id 0
        Ok(()) => {}
        Err(e) => {
            return ApproveCollectionResult::Err(ApproveCollectionError::GenericError {
                error_code: Nat::from(0u64),
                message: e,
            });
        }
    }

    match verify_approval_timing(arg.approval_info.created_at_time, current_time) {
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
        subaccount: arg.approval_info.from_subaccount,
    };

    let anonymous_account = Account {
        owner: Principal::anonymous(),
        subaccount: None,
    };

    if arg.approval_info.spender == anonymous_account {
        return ApproveCollectionResult::Err(ApproveCollectionError::InvalidSpender);
    }

    let has_nfts = read_state(|state| {
        state
            .data
            .tokens_list
            .iter()
            .any(|(_, token)| token.token_owner.owner == caller)
    });

    if !has_nfts {
        return ApproveCollectionResult::Err(ApproveCollectionError::GenericError {
            error_code: Nat::from(1u64),
            message: "Caller must own at least one NFT to approve collection".to_string(),
        });
    }

    let memo_clone = arg.approval_info.memo.clone();

    let approval = Approval {
        spender: WrappedAccount::from(arg.approval_info.spender.clone()),
        from: WrappedAccount::from(from_account.clone()),
        expires_at: arg.approval_info.expires_at,
        created_at: current_time,
        memo: memo_clone.clone().map(|m| m.to_vec()),
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

    let would_exceed_max_approvals = __COLLECTION_APPROVALS.with_borrow(|collection_approvals| {
        collection_approvals
            .get(&WrappedAccount::from(from_account.clone()))
            .is_some()
            && collection_approvals
                .get(&WrappedAccount::from(from_account.clone()))
                .unwrap()
                .0
                .len()
                >= max_approvals_per_collection
    });

    if would_exceed_max_approvals {
        return ApproveCollectionResult::Err(ApproveCollectionError::GenericError {
            error_code: Nat::from(1u64),
            message: "Maximum approvals per collection exceeded".to_string(),
        });
    }

    let transaction = ICRC37Transaction::new(
        "37approve_coll".to_string(),
        current_time,
        ICRC37TransactionData {
            op: "37approve_coll".to_string(),
            tid: None,
            from: Some(from_account.clone()),
            to: None,
            memo: memo_clone.clone(),
            created_at_time: Some(Nat::from(arg.approval_info.created_at_time)),
            spender: Some(arg.approval_info.spender),
            exp: arg.approval_info.expires_at.map(|e| Nat::from(e)),
        },
    );

    let index = match icrc3_add_transaction(transaction) {
        Ok(index) => index,
        Err(e) => {
            return ApproveCollectionResult::Err(ApproveCollectionError::GenericError {
                error_code: Nat::from(1u64),
                message: format!("Failed to log transaction: {}", e),
            });
        }
    };

    __COLLECTION_APPROVALS.with_borrow_mut(|collection_approvals| {
        let op_approval_map = collection_approvals.get(&WrappedAccount::from(from_account.clone()));

        let mut approval_map = if op_approval_map.is_none() {
            WrappedApprovalValue(HashMap::new())
        } else {
            op_approval_map.unwrap()
        };

        approval_map.0.insert(
            WrappedAccount::from(arg.approval_info.spender.clone()),
            approval,
        );

        collection_approvals.insert(WrappedAccount::from(from_account.clone()), approval_map);
    });

    ApproveCollectionResult::Ok(Nat::from(index))
}

#[update]
fn icrc37_revoke_token_approvals(
    args: icrc37_revoke_token_approvals::Args,
) -> icrc37_revoke_token_approvals::Response {
    let caller = ic_cdk::api::msg_caller();

    match guard_sliding_window(args[0].token_id.clone()) {
        Err(e) => {
            return icrc37_revoke_token_approvals::Response::Err(
                icrc37_revoke_token_approvals::RevokeTokenApprovalError::GenericError {
                    error_code: Nat::from(0u64),
                    message: e,
                },
            );
        }
        Ok(()) => {}
    }

    // here we check the max revoke approvals,
    // note that if spender is not provided, we will revoke all approvals for the token
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

    Ok(args
        .into_iter()
        .take(max_revoke_approvals)
        .map(|arg| {
            let current_time = ic_cdk::api::time();
            Some(revoke_token_approvals(arg, caller, current_time))
        })
        .collect())
}

fn revoke_token_approvals(
    arg: icrc37_revoke_token_approvals::RevokeTokenApprovalArg,
    caller: Principal,
    current_time: u64,
) -> icrc37_revoke_token_approvals::RevokeTokenApprovalResponse {
    use icrc37_revoke_token_approvals::{RevokeTokenApprovalError, RevokeTokenApprovalResponse};

    if let Some(created_at_time) = arg.created_at_time {
        match verify_approval_timing(created_at_time, current_time) {
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

    let token_approvals = __TOKEN_APPROVALS
        .with_borrow(|token_approvals| token_approvals.get(&WrappedNat::from(arg.token_id.clone())));

    if token_approvals.is_none() {
        return RevokeTokenApprovalResponse::Err(RevokeTokenApprovalError::ApprovalDoesNotExist);
    }

    let mut approval_map = token_approvals.unwrap().0;

    if let Some(spender) = &arg.spender {
        approval_map.remove(&WrappedAccount::from(spender.clone()));
    } else {
        approval_map.clear();
    };

    let created_at_time = arg.created_at_time.map(|t| Nat::from(t));

    let transaction = ICRC37Transaction::new(
        "37revoke".to_string(),
        current_time,
        ICRC37TransactionData {
            op: "37revoke".to_string(),
            tid: Some(arg.token_id.clone()),
            from: Some(from_account.clone()),
            to: None,
            memo: arg.memo,
            created_at_time: created_at_time,
            spender: arg.spender,
            exp: None,
        },
    );

    let index = match icrc3_add_transaction(transaction) {
        Ok(index) => index,
        Err(e) => {
            return RevokeTokenApprovalResponse::Err(RevokeTokenApprovalError::GenericError {
                error_code: Nat::from(1u64),
                message: format!("Failed to log transaction: {}", e),
            });
        }
    };

    __TOKEN_APPROVALS.with_borrow_mut(|token_approvals| {
        token_approvals.insert(
            WrappedNat::from(arg.token_id.clone()),
            WrappedApprovalValue(approval_map),
        );
    });

    RevokeTokenApprovalResponse::Ok(Nat::from(index))
}

#[update]
fn icrc37_revoke_collection_approvals(
    args: icrc37_revoke_collection_approvals::Args,
) -> icrc37_revoke_collection_approvals::Response {
    let caller = ic_cdk::api::msg_caller();

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

    Ok(args
        .into_iter()
        .take(max_revoke_approvals)
        .map(|arg| {
            let current_time = ic_cdk::api::time();
            Some(revoke_collection_approvals(arg, caller, current_time))
        })
        .collect())
}

fn revoke_collection_approvals(
    arg: icrc37_revoke_collection_approvals::RevokeCollectionApprovalArg,
    caller: Principal,
    current_time: u64,
) -> icrc37_revoke_collection_approvals::RevokeCollectionApprovalResult {
    use icrc37_revoke_collection_approvals::{
        RevokeCollectionApprovalError, RevokeCollectionApprovalResult,
    };

    match guard_sliding_window(candid::Nat::from(0u64)) {
        // we consider the collection as a token with id 0
        Ok(()) => {}
        Err(e) => {
            return RevokeCollectionApprovalResult::Err(
                RevokeCollectionApprovalError::GenericError {
                    error_code: Nat::from(0u64),
                    message: e,
                },
            );
        }
    }

    if let Some(created_at_time) = arg.created_at_time {
        match verify_approval_timing(created_at_time, current_time) {
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

    let collection_approvals = __COLLECTION_APPROVALS.with_borrow(|collection_approvals| {
        collection_approvals.get(&WrappedAccount::from(from_account.clone()))
    });

    if collection_approvals.is_none() {
        return RevokeCollectionApprovalResult::Err(
            RevokeCollectionApprovalError::ApprovalDoesNotExist,
        );
    }

    let mut approval_map = collection_approvals.unwrap().0;

    if let Some(spender) = &arg.spender {
        approval_map.remove(&WrappedAccount::from(spender.clone()));
    } else {
        approval_map.clear();
    }

    let created_at_time = arg.created_at_time.map(|t| Nat::from(t));

    let transaction = ICRC37Transaction::new(
        "37revoke_coll".to_string(),
        current_time,
        ICRC37TransactionData {
            op: "37revoke_coll".to_string(),
            tid: None,
            from: Some(from_account.clone()),
            to: None,
            memo: arg.memo,
            created_at_time: created_at_time,
            spender: arg.spender.clone(),
            exp: None,
        },
    );

    let index = match icrc3_add_transaction(transaction) {
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

    __COLLECTION_APPROVALS.with_borrow_mut(|collection_approvals| {
        collection_approvals.insert(
            WrappedAccount::from(from_account.clone()),
            WrappedApprovalValue(approval_map),
        );
    });

    RevokeCollectionApprovalResult::Ok(Nat::from(index))
}

#[update]
fn icrc37_transfer_from(args: icrc37_transfer_from::Args) -> icrc37_transfer_from::Response {
    let caller = ic_cdk::api::msg_caller();

    let mut results = Vec::with_capacity(args.len());

    for arg in args {
        let current_time = ic_cdk::api::time();
        let result = transfer_from(arg, caller, current_time);
        results.push(Some(result));
    }

    Ok(results)
}

fn transfer_from(
    arg: icrc37_transfer_from::TransferFromArg,
    caller: Principal,
    current_time: u64,
) -> icrc37_transfer_from::TransferFromResult {
    use icrc37_transfer_from::{TransferFromError, TransferFromResult};

    match guard_sliding_window(arg.token_id.clone()) {
        Ok(()) => {}
        Err(e) => {
            return TransferFromResult::Err(TransferFromError::GenericError {
                error_code: Nat::from(0u64),
                message: e,
            });
        }
    }

    if let Some(created_at_time) = arg.created_at_time {
        match verify_approval_timing(created_at_time, current_time) {
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

    let has_token_approval = __TOKEN_APPROVALS.with_borrow_mut(|token_approvals| {
        if let Some(token_approval) = token_approvals.get(&WrappedNat::from(arg.token_id.clone())) {
            let mut approval_map = token_approval.0;

            if let Some(approval) = approval_map.get(&WrappedAccount::from(spender_account)) {
                if let Some(expires_at) = approval.expires_at {
                    if expires_at <= current_time {
                        approval_map.remove(&WrappedAccount::from(spender_account));
                        token_approvals.insert(
                            WrappedNat::from(arg.token_id.clone()),
                            WrappedApprovalValue(approval_map),
                        );
                        return false;
                    }
                    return true;
                }
                return true;
            }
        }
        false
    });

    let has_collection_approval = __COLLECTION_APPROVALS.with_borrow_mut(|collection_approvals| {
        if let Some(token_owner_approvals) =
            collection_approvals.get(&WrappedAccount::from(nft.token_owner.clone()))
        {
            let mut approval_map = token_owner_approvals.0;

            if let Some(approval) = approval_map.get(&WrappedAccount::from(spender_account.clone()))
            {
                if let Some(expires_at) = approval.expires_at {
                    // remove the approval if it has expired
                    if expires_at <= current_time {
                        approval_map.remove(&WrappedAccount::from(spender_account));
                        if approval_map.is_empty() {
                            collection_approvals.remove(&WrappedAccount::from(nft.token_owner));
                        }

                        collection_approvals.insert(
                            WrappedAccount::from(nft.token_owner.clone()),
                            WrappedApprovalValue(approval_map),
                        );
                        return false;
                    }
                    return true;
                }
                return true;
            }
        }
        false
    });

    let is_caller_token_holder = spender_account == arg.from;

    if !is_owner || (!is_caller_token_holder && !has_token_approval && !has_collection_approval) {
        return TransferFromResult::Err(TransferFromError::Unauthorized);
    }

    let transaction = ICRC37Transaction::new(
        "37xfer".to_string(),
        current_time,
        ICRC37TransactionData {
            op: "37xfer".to_string(),
            tid: Some(arg.token_id.clone()),
            from: Some(arg.from.clone()),
            to: Some(arg.to.clone()),
            memo: arg.memo,
            created_at_time: arg.created_at_time.map(Nat::from),
            spender: Some(spender_account),
            exp: None,
        },
    );

    let index = match icrc3_add_transaction(transaction) {
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
    });

    __TOKEN_APPROVALS.with_borrow_mut(|token_approvals| {
        token_approvals.remove(&WrappedNat::from(arg.token_id.clone()));
    });

    let has_nfts = __TOKEN_APPROVALS.with_borrow(|token_approvals| {
        token_approvals
            .get(&WrappedNat::from(arg.token_id.clone()))
            .is_some()
    });

    if !has_nfts {
        __COLLECTION_APPROVALS.with_borrow_mut(|collection_approvals| {
            collection_approvals.remove(&WrappedAccount::from(arg.from.clone()));
        });
    }

    TransferFromResult::Ok(Nat::from(index))
}
