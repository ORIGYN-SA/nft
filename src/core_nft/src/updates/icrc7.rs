use crate::utils::check_memo;
use crate::utils::trace;
use crate::{
    state::{icrc3_add_transaction, mutate_state, read_state},
    types::icrc7,
    types::transaction::Icrc3Transaction,
};
use candid::{Nat, Principal};
use ic_cdk_macros::update;

fn transfer_nft(arg: &icrc7::TransferArg) -> Result<Nat, icrc7::icrc7_transfer::TransferError> {
    let mut nft = mutate_state(|state| state.data.tokens_list.get(&arg.token_id).cloned())
        .ok_or(icrc7::icrc7_transfer::TransferError::NonExistingTokenId)?;

    check_memo(arg.memo.clone()).map_err(|e| {
        icrc7::icrc7_transfer::TransferError::GenericError {
            error_code: Nat::from(0u64),
            message: e,
        }
    })?;

    if nft.token_owner.owner != ic_cdk::caller()
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

    match icrc3_add_transaction(transaction.clone()) {
        Ok(transaction_id) => {
            nft.transfer(arg.to.clone());
            mutate_state(|state| state.data.update_token_by_id(&nft.token_id, &nft));
            Ok(Nat::from(transaction_id))
        }
        Err(e) => {
            if let bity_ic_icrc3::types::Icrc3Error::DuplicateTransaction { duplicate_of } = e {
                return Err(icrc7::icrc7_transfer::TransferError::Duplicate {
                    duplicate_of: Nat::from(duplicate_of),
                });
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
        return vec![Some(Err(
            icrc7::icrc7_transfer::TransferError::GenericError {
                error_code: Nat::from(0u64),
                message: "No argument provided".to_string(),
            },
        ))];
    }

    let (max_update_batch_size, _, _, _) = read_state(|state| {
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
        return vec![Some(Err(
            icrc7::icrc7_transfer::TransferError::GenericError {
                error_code: Nat::from(0u64),
                message: "Exceed Max allowed Update Batch Size".to_string(),
            },
        ))];
    }

    if ic_cdk::caller() == Principal::anonymous() {
        return vec![Some(Err(
            icrc7::icrc7_transfer::TransferError::GenericError {
                error_code: Nat::from(0u64),
                message: "Anonymous caller not allowed to transfer".to_string(),
            },
        ))];
    }

    args.iter()
        .take(max_batch_size)
        .map(transfer_nft)
        .map(Some)
        .collect()
}
