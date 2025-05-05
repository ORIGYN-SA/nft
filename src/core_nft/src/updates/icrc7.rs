use crate::types::nft;
use crate::utils::check_memo;
use crate::utils::trace;
use crate::{
    state::{icrc3_add_transaction, mutate_state, read_state},
    types::icrc7,
    types::transaction::Icrc3Transaction,
};
use candid::{Nat, Principal};
use ic_cdk::api::call::RejectionCode;
use ic_cdk_macros::update;
use icrc_ledger_types::icrc1::account::Account;

#[update]
pub async fn icrc7_transfer(args: icrc7::icrc7_transfer::Args) -> icrc7::icrc7_transfer::Response {
    if args.is_empty() {
        return vec![Some(Err((
            RejectionCode::CanisterError,
            "No argument provided".to_string(),
        )))];
    }

    let (max_update_batch_size, atomic_batch_transfers, tx_window, permitted_drift) =
        read_state(|state| {
            (
                state
                    .data
                    .max_update_batch_size
                    .clone()
                    .unwrap_or(Nat::from(icrc7::DEFAULT_MAX_UPDATE_BATCH_SIZE)),
                state.data.atomic_batch_transfers.unwrap_or(false),
                state.data.tx_window.clone(),
                state.data.permitted_drift.clone(),
            )
        });

    let max_batch_size = usize::try_from(max_update_batch_size.0).unwrap();
    if args.len() > max_batch_size {
        return vec![Some(Err((
            RejectionCode::CanisterError,
            "Exceed Max allowed Update Batch Size".to_string(),
        )))];
    }

    if ic_cdk::caller() == Principal::anonymous() {
        return vec![Some(Err((
            RejectionCode::CanisterError,
            "Anonymous Identity".to_string(),
        )))];
    }

    let current_time = ic_cdk::api::time();
    let mut txn_results = vec![None; args.len()];
    for (index, arg) in args.iter().enumerate() {
        let mut nft: nft::Icrc7Token =
            match mutate_state(|state| state.data.tokens_list.get(&arg.token_id).cloned()) {
                Some(token) => token,
                None => {
                    txn_results[index] = Some(Err((
                        RejectionCode::CanisterError,
                        "Token does not exist".to_string(),
                    )));
                    continue;
                }
            };

        match check_memo(arg.memo.clone()) {
            Ok(_) => {}
            Err(e) => {
                txn_results[index] = Some(Err((RejectionCode::CanisterError, e)));
                continue;
            }
        }

        let caller_as_principal = ic_cdk::caller();

        if nft.token_owner.owner != caller_as_principal {
            txn_results[index] = Some(Err((
                RejectionCode::CanisterError,
                "Token owner does not match the sender".to_string(),
            )));
            continue;
        }

        let anonymous_account = Account {
            owner: Principal::anonymous(),
            subaccount: None,
        };

        if arg.to == anonymous_account {
            txn_results[index] = Some(Err((
                RejectionCode::CanisterError,
                "Invalid recipient".to_string(),
            )));
            continue;
        }

        if nft.token_owner == arg.to {
            txn_results[index] = Some(Err((
                RejectionCode::CanisterError,
                "Cannot transfer to the same owner".to_string(),
            )));
            continue;
        }

        let time = arg.created_at_time.unwrap_or(current_time);
        trace(&format!("time: {:?}", time));

        let drift = permitted_drift
            .clone()
            .map(|d| u64::try_from(d.0).unwrap())
            .unwrap_or(icrc7::DEFAULT_PERMITTED_DRIFT);

        if time > current_time + drift {
            txn_results[index] = Some(Err((
                RejectionCode::CanisterError,
                format!("CreatedInFuture {{ ledger_time: {} }}", current_time),
            )));
            continue;
        }

        let tx_window = tx_window
            .clone()
            .map(|d| u64::try_from(d.0).unwrap())
            .unwrap_or(icrc7::DEFAULT_TX_WINDOW);

        if time < current_time.saturating_sub(tx_window + drift) {
            txn_results[index] = Some(Err((RejectionCode::CanisterError, "TooOld".to_string())));
            continue;
        }

        nft.transfer(arg.to.clone());

        let transaction = Icrc3Transaction {
            btype: "7xfer".to_string(),
            timestamp: current_time,
            tx: crate::types::transaction::TransactionData {
                tid: Some(arg.token_id.clone()),
                from: Some(nft.token_owner.clone()),
                to: Some(arg.to.clone()),
                meta: None,
                memo: arg.memo.clone(),
                created_at_time: Some(Nat::from(time)),
                spender: None,
                exp: None,
            },
        };

        match icrc3_add_transaction(transaction).await {
            Ok(_) => {
                txn_results[index] = Some(Ok(()));
            }
            Err(e) => {
                txn_results[index] = Some(Err((
                    RejectionCode::CanisterError,
                    format!("Failed to log transaction: {}", e),
                )));
            }
        }

        mutate_state(|state| state.data.update_token_by_id(&nft.token_id, &nft));
    }

    txn_results
}
