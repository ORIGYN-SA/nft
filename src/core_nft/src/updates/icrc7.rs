use ic_cdk::api::call::RejectionCode;
use ic_cdk_macros::update;
use crate::{ state::{ read_state, mutate_state }, types::icrc7 };
use candid::{ Nat, Principal };
use crate::types::nft;
use icrc_ledger_types::icrc1::account::Account;
use crate::utils::check_memo;

#[update]
pub fn icrc7_transfer(args: icrc7::icrc7_transfer::Args) -> icrc7::icrc7_transfer::Response {
    if args.len() == 0 {
        return vec![Some(Err((RejectionCode::CanisterError, "No argument provided".to_string())))];
    }

    let max_update_batch_size: usize = read_state(|state| {
        let nat_max_update_batch_size = state.data.max_query_batch_size
            .clone()
            .unwrap_or(Nat::from(icrc7::DEFAULT_MAX_UPDATE_BATCH_SIZE));
        usize::try_from(nat_max_update_batch_size.0).unwrap()
    });

    if args.len() > max_update_batch_size {
        return vec![
            Some(
                Err((
                    RejectionCode::CanisterError,
                    "Exceed Max allowed Update Batch Size".to_string(),
                ))
            )
        ];
    }

    if ic_cdk::caller() == Principal::anonymous() {
        return vec![Some(Err((RejectionCode::CanisterError, "Anonymous Identity".to_string())))];
    }

    let current_time = ic_cdk::api::time();
    let mut txn_results = vec![None; args.len()];
    for (index, arg) in args.iter().enumerate() {
        let mut nft: nft::Icrc7Token = match
            mutate_state(|state| state.data.tokens_list.get(&arg.token_id).cloned())
        {
            Some(token) => token,
            None => {
                txn_results[index] = Some(
                    Err((RejectionCode::CanisterError, "Token does not exist".to_string()))
                );
                continue;
            }
        };

        match check_memo(arg.memo.clone()) {
            Ok(_) => {}
            Err(e) => {
                txn_results[index] = Some(Err((RejectionCode::CanisterError, e)));
            }
        }

        let caller_as_account = Account { owner: ic_cdk::caller(), subaccount: None };
        if nft.token_owner != caller_as_account {
            txn_results[index] = Some(
                Err((
                    RejectionCode::CanisterError,
                    "Token owner does not match the sender".to_string(),
                ))
            );
            continue;
        }

        if nft.token_owner == arg.to {
            txn_results[index] = Some(
                Err((RejectionCode::CanisterError, "Cannot transfer to the same owner".to_string()))
            );
            continue;
        }

        let time = arg.created_at_time.unwrap_or(current_time);

        nft.transfer(arg.to.clone());

        mutate_state(|state| state.data.update_token_by_id(&nft.token_id, &nft));

        // let txn_id = log_transaction();
        // txn_results[index] = Some(Ok(txn_id));
    }
    return txn_results;
}
