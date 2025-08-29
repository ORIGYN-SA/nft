use crate::utils::check_memo;
use crate::utils::trace;
use crate::{
    state::{icrc3_add_transaction, mutate_state, read_state},
    types::icrc7,
};
use bity_ic_icrc3::transaction::{ICRC7Transaction, ICRC7TransactionData};
use candid::{Nat, Principal};
use ic_cdk_macros::update;

use crate::guards::guard_sliding_window;

fn transfer_nft(arg: &icrc7::TransferArg) -> Result<Nat, icrc7::icrc7_transfer::TransferError> {
    guard_sliding_window(arg.token_id.clone()).map_err(|e| {
        icrc7::icrc7_transfer::TransferError::GenericError {
            error_code: Nat::from(0u64),
            message: e,
        }
    })?;

    let mut nft = mutate_state(|state| state.data.tokens_list.get(&arg.token_id).cloned())
        .ok_or(icrc7::icrc7_transfer::TransferError::NonExistingTokenId)?;

    check_memo(arg.memo.clone()).map_err(|e| {
        icrc7::icrc7_transfer::TransferError::GenericError {
            error_code: Nat::from(0u64),
            message: e,
        }
    })?;

    if nft.token_owner.owner != ic_cdk::api::msg_caller()
        || arg.to.owner == Principal::anonymous()
        || nft.token_owner == arg.to
    {
        return Err(icrc7::icrc7_transfer::TransferError::InvalidRecipient);
    }

    let current_time = ic_cdk::api::time();
    let time = arg.created_at_time.unwrap_or(current_time);
    trace(&format!("time: {:?}", time));

    let (permitted_drift, tx_window) = read_state(|state| {
        (
            state.data.permitted_drift.clone(),
            state.data.tx_window.clone(),
        )
    });

    let drift = permitted_drift
        .map(|d| u64::try_from(d.0).unwrap())
        .unwrap_or(icrc7::DEFAULT_PERMITTED_DRIFT);

    if time > current_time + drift {
        return Err(icrc7::icrc7_transfer::TransferError::CreatedInFuture {
            ledger_time: Nat::from(current_time),
        });
    }

    let tx_window = tx_window
        .map(|d| u64::try_from(d.0).unwrap())
        .unwrap_or(icrc7::DEFAULT_TX_WINDOW);

    if time < current_time.saturating_sub(tx_window + drift) {
        return Err(icrc7::icrc7_transfer::TransferError::TooOld);
    }

    let transaction = ICRC7Transaction::new(
        "7xfer".to_string(),
        current_time,
        ICRC7TransactionData {
            op: "7xfer".to_string(),
            tid: Some(arg.token_id.clone()),
            from: Some(nft.token_owner.clone()),
            to: Some(arg.to.clone()),
            meta: None,
            memo: arg.memo.clone(),
            created_at_time: Some(Nat::from(time)),
        },
    );

    let previous_owner = nft.token_owner.clone();

    // this is safe to do this as they is no await in the method, meaning state is committed at the end of the icrc7_transfer method.
    match icrc3_add_transaction(transaction.clone()) {
        Ok(transaction_id) => {
            nft.transfer(arg.to.clone());
            mutate_state(|state| {
                state.data.update_token_by_id(&nft.token_id, &nft);
                state
                    .data
                    .tokens_list_by_owner
                    .entry(arg.to.clone())
                    .or_insert(vec![])
                    .push(nft.token_id.clone());
                state
                    .data
                    .tokens_list_by_owner
                    .entry(previous_owner)
                    .or_insert(vec![])
                    .retain(|id| *id != nft.token_id.clone());
            });
            Ok(Nat::from(transaction_id))
        }
        Err(e) => {
            if let bity_ic_icrc3::types::Icrc3Error::DuplicateTransaction { duplicate_of } = e {
                return Ok(Nat::from(duplicate_of));
            }
            Err(icrc7::icrc7_transfer::TransferError::GenericError {
                error_code: Nat::from(2u64),
                message: format!("Failed to log transaction: {}", e),
            })
        }
    }
}

#[update]
pub async fn icrc7_transfer(args: icrc7::icrc7_transfer::Args) -> icrc7::icrc7_transfer::Response {
    if args.is_empty() {
        return vec![];
    }

    let (max_update_batch_size, atomic_batch_transfers) = read_state(|state| {
        (
            state
                .data
                .max_update_batch_size
                .clone()
                .unwrap_or(Nat::from(icrc7::DEFAULT_MAX_UPDATE_BATCH_SIZE)),
            state.data.atomic_batch_transfers.unwrap_or(false),
        )
    });

    let max_batch_size = usize::try_from(max_update_batch_size.0).unwrap();

    if atomic_batch_transfers && args.len() > max_batch_size {
        return vec![Some(Err(
            icrc7::icrc7_transfer::TransferError::GenericError {
                error_code: Nat::from(0u64),
                message: "Exceed Max allowed Update Batch Size".to_string(),
            },
        ))];
    }

    if ic_cdk::api::msg_caller() == Principal::anonymous() {
        return vec![Some(Err(
            icrc7::icrc7_transfer::TransferError::GenericError {
                error_code: Nat::from(0u64),
                message: "Anonymous caller not allowed to transfer".to_string(),
            },
        ))];
    }

    // Process only up to max_batch_size transfers
    args.iter()
        .take(max_batch_size)
        .map(transfer_nft)
        .map(Some)
        .collect()
}
