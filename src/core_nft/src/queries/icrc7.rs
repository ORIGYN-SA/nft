use crate::types::icrc7;
use candid::Nat;
use ic_cdk::update;
use ic_cdk_macros::query;
use icrc_ledger_types::icrc::generic_value::ICRC3Value;

use crate::state::read_state;

#[query]
pub fn icrc7_collection_metadata() -> icrc7::icrc7_collection_metadata::Response {
    read_state(|state| {
        let mut metadata = Vec::new();

        metadata.push((
            "icrc7:symbol".to_string(),
            ICRC3Value::Text(state.data.symbol.clone()),
        ));
        metadata.push((
            "icrc7:name".to_string(),
            ICRC3Value::Text(state.data.name.clone()),
        ));

        metadata.push((
            "icrc7:total_supply".to_string(),
            ICRC3Value::Nat(state.data.total_supply()),
        ));

        if let Some(description) = &state.data.description {
            metadata.push((
                "icrc7:description".to_string(),
                ICRC3Value::Text(description.clone()),
            ));
        }

        if state.data.logo.is_some() {
            metadata.push((
                "icrc7:logo".to_string(),
                ICRC3Value::Text(format!(
                    "https://{}.raw.icp0.io/logo",
                    ic_cdk::id().to_text()
                )),
            ));
        }

        if let Some(supply_cap) = &state.data.supply_cap {
            metadata.push((
                "icrc7:supply_cap".to_string(),
                ICRC3Value::Nat(supply_cap.clone()),
            ));
        }

        if let Some(max_query_batch_size) = &state.data.max_query_batch_size {
            metadata.push((
                "icrc7:max_query_batch_size".to_string(),
                ICRC3Value::Nat(max_query_batch_size.clone()),
            ));
        }

        if let Some(max_update_batch_size) = &state.data.max_update_batch_size {
            metadata.push((
                "icrc7:max_update_batch_size".to_string(),
                ICRC3Value::Nat(max_update_batch_size.clone()),
            ));
        }

        if let Some(default_take_value) = &state.data.default_take_value {
            metadata.push((
                "icrc7:default_take_value".to_string(),
                ICRC3Value::Nat(default_take_value.clone()),
            ));
        }

        if let Some(max_take_value) = &state.data.max_take_value {
            metadata.push((
                "icrc7:max_take_value".to_string(),
                ICRC3Value::Nat(max_take_value.clone()),
            ));
        }

        if let Some(max_memo_size) = &state.data.max_memo_size {
            metadata.push((
                "icrc7:max_memo_size".to_string(),
                ICRC3Value::Nat(max_memo_size.clone()),
            ));
        }

        if let Some(atomic_batch_transfers) = &state.data.atomic_batch_transfers {
            metadata.push((
                "icrc7:atomic_batch_transfers".to_string(),
                ICRC3Value::Text(atomic_batch_transfers.to_string()),
            ));
        }

        if let Some(tx_window) = &state.data.tx_window {
            metadata.push((
                "icrc7:tx_window".to_string(),
                ICRC3Value::Nat(tx_window.clone()),
            ));
        }

        if let Some(permitted_drift) = &state.data.permitted_drift {
            metadata.push((
                "icrc7:permitted_drift".to_string(),
                ICRC3Value::Nat(permitted_drift.clone()),
            ));
        }

        metadata.sort_by(|a, b| a.0.cmp(&b.0));
        metadata
    })
}

#[query]
pub fn icrc7_symbol() -> icrc7::icrc7_symbol::Response {
    read_state(|state| state.data.symbol.clone())
}

#[query]
pub fn icrc7_name() -> icrc7::icrc7_name::Response {
    read_state(|state| state.data.name.clone())
}

#[query]
pub fn icrc7_description() -> icrc7::icrc7_description::Response {
    read_state(|state| state.data.description.clone())
}

#[query]
pub fn icrc7_logo() -> icrc7::icrc7_logo::Response {
    if let Some(_) = read_state(|state| state.data.logo.clone()) {
        Some(format!(
            "https://{}.raw.icp0.io/logo",
            ic_cdk::id().to_text()
        ))
    } else {
        None
    }
}

#[query]
pub fn icrc7_total_supply() -> icrc7::icrc7_total_supply::Response {
    read_state(|state| state.data.total_supply())
}

#[query]
pub fn icrc7_supply_cap() -> icrc7::icrc7_supply_cap::Response {
    read_state(|state| state.data.supply_cap.clone())
}

#[query]
pub fn icrc7_max_query_batch_size() -> icrc7::icrc7_max_query_batch_size::Response {
    read_state(|state| state.data.max_query_batch_size.clone())
}

#[query]
pub fn icrc7_max_update_batch_size() -> icrc7::icrc7_max_update_batch_size::Response {
    read_state(|state| state.data.max_update_batch_size.clone())
}

#[query]
pub fn icrc7_default_take_value() -> icrc7::icrc7_default_take_value::Response {
    read_state(|state| state.data.default_take_value.clone())
}

#[query]
pub fn icrc7_max_take_value() -> icrc7::icrc7_max_take_value::Response {
    read_state(|state| state.data.max_take_value.clone())
}

#[query]
pub fn icrc7_max_memo_size() -> icrc7::icrc7_max_memo_size::Response {
    read_state(|state| state.data.max_memo_size.clone())
}

#[query]
pub fn icrc7_atomic_batch_transfers() -> icrc7::icrc7_atomic_batch_transfers::Response {
    read_state(|state| state.data.atomic_batch_transfers.clone())
}

#[query]
pub fn icrc7_tx_window() -> icrc7::icrc7_tx_window::Response {
    read_state(|state| state.data.tx_window.clone())
}

#[query]
pub fn icrc7_permitted_drift() -> icrc7::icrc7_permitted_drift::Response {
    read_state(|state| state.data.permitted_drift.clone())
}

#[update]
pub async fn icrc7_token_metadata(
    token_ids: icrc7::icrc7_token_metadata::Args,
) -> icrc7::icrc7_token_metadata::Response {
    let mut ret = Vec::new();
    for token_id in token_ids {
        let token = read_state(|state| state.data.get_token_by_id(&token_id).cloned());
        match token {
            Some(token) => {
                let metadata = token.token_metadata().await;
                let mut metadata_vec: Vec<(String, ICRC3Value)> = metadata.into_iter().collect();
                metadata_vec.sort_by(|a, b| a.0.cmp(&b.0));
                ret.push(Some(metadata_vec));
            }
            None => {
                ret.push(None);
            }
        }
    }
    ret
}

#[query]
pub fn icrc7_owner_of(token_ids: icrc7::icrc7_owner_of::Args) -> icrc7::icrc7_owner_of::Response {
    read_state(|state| {
        token_ids
            .iter()
            .map(|token_id| state.data.owner_of(token_id))
            .collect()
    })
}

#[query]
pub fn icrc7_balance_of(
    accounts: icrc7::icrc7_balance_of::Args,
) -> icrc7::icrc7_balance_of::Response {
    read_state(|state| {
        accounts
            .iter()
            .map(|account| state.data.tokens_balance_of(account))
            .collect()
    })
}

#[query]
pub fn icrc7_tokens(args: icrc7::icrc7_tokens::Args) -> icrc7::icrc7_tokens::Response {
    read_state(|state| {
        let prev = args.0.unwrap_or(Nat::from(0 as u64));
        let take: usize = usize::try_from(
            args.1
                .unwrap_or_else(|| {
                    state
                        .data
                        .default_take_value
                        .clone()
                        .unwrap_or(Nat::from(icrc7::DEFAULT_TAKE_VALUE))
                })
                .0,
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
pub fn icrc7_tokens_of(args: icrc7::icrc7_tokens_of::Args) -> icrc7::icrc7_tokens_of::Response {
    read_state(|state| {
        let (account, prev, take) = args;
        let prev = prev.unwrap_or(Nat::from(0 as u64));
        let take: usize = usize::try_from(
            take.unwrap_or_else(|| {
                state
                    .data
                    .default_take_value
                    .clone()
                    .unwrap_or(Nat::from(icrc7::DEFAULT_TAKE_VALUE))
            })
            .0,
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
