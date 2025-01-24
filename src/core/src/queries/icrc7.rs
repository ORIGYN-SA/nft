use ic_cdk_macros::query;
use crate::types::icrc7;
use candid::Nat;

use crate::state::read_state;

#[query]
pub fn icrc7_collection_metadata() -> icrc7::CollectionMetadataResult {
    // TODO
    Vec::new()
}

#[query]
pub fn icrc7_symbol() -> icrc7::SymbolResult {
    read_state(|state| state.data.symbol.clone())
}

#[query]
pub fn icrc7_name() -> icrc7::NameResult {
    read_state(|state| state.data.name.clone())
}

#[query]
pub fn icrc7_description() -> icrc7::DescriptionResult {
    read_state(|state| state.data.description.clone())
}

#[query]
pub fn icrc7_logo() -> icrc7::LogoResult {
    if let Some(_) = read_state(|state| state.data.logo.clone()) {
        Some(format!("https://{}.raw.icp0.io/logo", ic_cdk::id().to_text()))
    } else {
        None
    }
}

#[query]
pub fn icrc7_total_supply() -> icrc7::TotalSupplyResult {
    read_state(|state| state.data.total_supply())
}

#[query]
pub fn icrc7_supply_cap() -> icrc7::SupplyCapResult {
    read_state(|state| state.data.supply_cap.clone())
}

#[query]
pub fn icrc7_max_query_batch_size() -> icrc7::MaxQueryBatchSizeResult {
    read_state(|state| state.data.max_query_batch_size.clone())
}

#[query]
pub fn icrc7_max_update_batch_size() -> icrc7::MaxUpdateBatchSizeResult {
    read_state(|state| state.data.max_update_batch_size.clone())
}

#[query]
pub fn icrc7_default_take_value() -> icrc7::DefaultTakeValueResult {
    read_state(|state| state.data.default_take_value.clone())
}

#[query]
pub fn icrc7_max_take_value() -> icrc7::MaxTakeValueResult {
    read_state(|state| state.data.max_take_value.clone())
}

#[query]
pub fn icrc7_max_memo_size() -> icrc7::MaxMemoSizeResult {
    read_state(|state| state.data.max_memo_size.clone())
}

#[query]
pub fn icrc7_atomic_batch_transfers() -> icrc7::AtomicBatchTransfersResult {
    read_state(|state| state.data.atomic_batch_transfers.clone())
}

#[query]
pub fn icrc7_tx_window() -> icrc7::TxWindowResult {
    read_state(|state| state.data.tx_window.clone())
}

#[query]
pub fn icrc7_permitted_drift() -> icrc7::PermittedDriftResult {
    read_state(|state| state.data.permitted_drift.clone())
}

#[query]
pub fn icrc7_token_metadata(token_ids: icrc7::TokenMetadataArgs) -> icrc7::TokenMetadataResult {
    read_state(|state| {
        token_ids
            .iter()
            .map(|token_id| {
                state.data
                    .get_token_by_id(token_id)
                    .map(|token| token.token_metadata().clone().into_iter().collect())
            })
            .collect()
    })
}

#[query]
pub fn icrc7_owner_of(token_ids: icrc7::OwnerOfArgs) -> icrc7::OwnerOfResult {
    read_state(|state| {
        token_ids
            .iter()
            .map(|token_id| state.data.owner_of(token_id))
            .collect()
    })
}

#[query]
pub fn icrc7_balance_of(accounts: icrc7::BalanceOfArgs) -> icrc7::BalanceOfResult {
    read_state(|state| {
        accounts
            .iter()
            .map(|account| state.data.tokens_balance_of(account))
            .collect()
    })
}

#[query]
pub fn icrc7_tokens(args: icrc7::TokensArgs) -> icrc7::TokensResult {
    read_state(|state| {
        let prev = args.0.unwrap_or(Nat::from(0 as u64));
        let take: usize = usize
            ::try_from(
                args.1.unwrap_or_else(||
                    state.data.default_take_value
                        .clone()
                        .unwrap_or(Nat::from(icrc7::DEFAULT_TAKE_VALUE))
                ).0
            )
            .unwrap_or(icrc7::DEFAULT_TAKE_VALUE);

        let mut tokens: Vec<_> = state.data.tokens_list.keys().cloned().collect();
        tokens.sort();
        let start_index = tokens
            .iter()
            .position(|id| id > &prev)
            .unwrap_or(tokens.len());
        tokens.into_iter().skip(start_index).take(take).collect()
    })
}

#[query]
pub fn icrc7_tokens_of(args: icrc7::TokensOfArgs) -> icrc7::TokensOfResult {
    read_state(|state| {
        let (account, prev, take) = args;
        let prev = prev.unwrap_or(Nat::from(0 as u64));
        let take: usize = usize
            ::try_from(
                take.unwrap_or_else(||
                    state.data.default_take_value
                        .clone()
                        .unwrap_or(Nat::from(icrc7::DEFAULT_TAKE_VALUE))
                ).0
            )
            .unwrap_or(icrc7::DEFAULT_TAKE_VALUE);

        let mut tokens: Vec<Nat> = state.data.tokens_ids_of_account(&account);
        tokens.sort();
        let start_index = tokens
            .iter()
            .position(|id| id > &prev)
            .unwrap_or(tokens.len());
        tokens.into_iter().skip(start_index).take(take).collect()
    })
}
