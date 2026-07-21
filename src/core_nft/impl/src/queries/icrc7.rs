use crate::state::read_state;
use crate::types::metadata::__METADATA;

use candid::Nat;
use ic_cdk_macros::query;
use icrc_ledger_types::icrc::generic_value::ICRC3Value;

#[query]
pub fn icrc7_collection_metadata() -> core_nft_api::queries::icrc7::CollectionMetadataResponse {
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

        if let Some(logo) = &state.data.logo {
            metadata.push(("icrc7:logo".to_string(), ICRC3Value::Text(logo.clone())));
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

        for (key, custom_val) in &state.data.custom_collection_metadata {
            metadata.push((key.clone(), custom_val.0.clone()));
        }

        metadata.sort_by(|a, b| a.0.cmp(&b.0));
        metadata
    })
}

#[query]
pub fn icrc7_symbol() -> core_nft_api::queries::icrc7::SymbolResponse {
    read_state(|state| state.data.symbol.clone())
}

#[query]
pub fn icrc7_name() -> core_nft_api::queries::icrc7::NameResponse {
    read_state(|state| state.data.name.clone())
}

#[query]
pub fn icrc7_description() -> core_nft_api::queries::icrc7::DescriptionResponse {
    read_state(|state| state.data.description.clone())
}

#[query]
pub fn icrc7_logo() -> core_nft_api::queries::icrc7::LogoResponse {
    read_state(|state| state.data.logo.clone())
}

#[query]
pub fn icrc7_total_supply() -> core_nft_api::queries::icrc7::TotalSupplyResponse {
    read_state(|state| state.data.total_supply())
}

#[query]
pub fn icrc7_supply_cap() -> core_nft_api::queries::icrc7::SupplyCapResponse {
    read_state(|state| state.data.supply_cap.clone())
}

#[query]
pub fn icrc7_max_query_batch_size() -> core_nft_api::queries::icrc7::MaxQueryBatchSizeResponse {
    read_state(|state| state.data.max_query_batch_size.clone())
}

#[query]
pub fn icrc7_max_update_batch_size() -> core_nft_api::queries::icrc7::MaxUpdateBatchSizeResponse {
    read_state(|state| state.data.max_update_batch_size.clone())
}

#[query]
pub fn icrc7_default_take_value() -> core_nft_api::queries::icrc7::DefaultTakeValueResponse {
    read_state(|state| state.data.default_take_value.clone())
}

#[query]
pub fn icrc7_max_take_value() -> core_nft_api::queries::icrc7::MaxTakeValueResponse {
    read_state(|state| state.data.max_take_value.clone())
}

#[query]
pub fn icrc7_max_memo_size() -> core_nft_api::queries::icrc7::MaxMemoSizeResponse {
    read_state(|state| state.data.max_memo_size.clone())
}

#[query]
pub fn icrc7_atomic_batch_transfers() -> core_nft_api::queries::icrc7::AtomicBatchTransfersResponse
{
    read_state(|state| state.data.atomic_batch_transfers.clone())
}

#[query]
pub fn icrc7_tx_window() -> core_nft_api::queries::icrc7::TxWindowResponse {
    read_state(|state| state.data.tx_window.clone())
}

#[query]
pub fn icrc7_permitted_drift() -> core_nft_api::queries::icrc7::PermittedDriftResponse {
    read_state(|state| state.data.permitted_drift.clone())
}

#[query]
pub fn icrc7_token_metadata(
    token_ids: core_nft_api::queries::icrc7::TokenMetadataArgs,
) -> core_nft_api::queries::icrc7::TokenMetadataResponse {
    let icrc7_max_query_batch_size = read_state(|state| state.data.max_query_batch_size.clone());
    let max_query_batch_size = icrc7_max_query_batch_size.unwrap_or(Nat::from(
        core_nft_common::types::icrc7::DEFAULT_MAX_QUERY_BATCH_SIZE,
    ));

    if token_ids.len()
        > usize::try_from(max_query_batch_size.0.clone())
            .unwrap_or(core_nft_common::types::icrc7::DEFAULT_MAX_QUERY_BATCH_SIZE as usize)
    {
        ic_cdk::trap(format!(
            "max_query_batch_size exceeded. Limit is {}. Retry with a smaller batch size.",
            max_query_batch_size.0
        ));
    }

    let mut ret = Vec::new();

    for token_id in token_ids {
        let token = read_state(|state| state.data.get_token_by_id(&token_id).cloned());
        match token {
            Some(token) => {
                let metadata = token.token_metadata(&__METADATA.with_borrow(|m| m.clone()));
                ret.push(Some(metadata));
            }
            None => {
                ret.push(None);
            }
        }
    }

    ret
}

#[query]
pub fn icrc7_owner_of(
    token_ids: core_nft_api::queries::icrc7::OwnerOfArgs,
) -> core_nft_api::queries::icrc7::OwnerOfResponse {
    let icrc7_max_query_batch_size = read_state(|state| state.data.max_query_batch_size.clone());
    let max_query_batch_size = icrc7_max_query_batch_size.unwrap_or(Nat::from(
        core_nft_common::types::icrc7::DEFAULT_MAX_QUERY_BATCH_SIZE,
    ));

    if token_ids.len()
        > usize::try_from(max_query_batch_size.0.clone())
            .unwrap_or(core_nft_common::types::icrc7::DEFAULT_MAX_QUERY_BATCH_SIZE as usize)
    {
        ic_cdk::trap(format!(
            "max_query_batch_size exceeded. Limit is {}. Retry with a smaller batch size.",
            max_query_batch_size.0
        ));
    }

    read_state(|state| {
        token_ids
            .iter()
            .map(|token_id| state.data.owner_of(token_id))
            .collect()
    })
}

#[query]
pub fn icrc7_balance_of(
    accounts: core_nft_api::queries::icrc7::BalanceOfArgs,
) -> core_nft_api::queries::icrc7::BalanceOfResponse {
    let icrc7_max_query_batch_size = read_state(|state| state.data.max_query_batch_size.clone());
    let max_query_batch_size = icrc7_max_query_batch_size.unwrap_or(Nat::from(
        core_nft_common::types::icrc7::DEFAULT_MAX_QUERY_BATCH_SIZE,
    ));

    if accounts.len()
        > usize::try_from(max_query_batch_size.0.clone())
            .unwrap_or(core_nft_common::types::icrc7::DEFAULT_MAX_QUERY_BATCH_SIZE as usize)
    {
        ic_cdk::trap(format!(
            "max_query_batch_size exceeded. Limit is {}. Retry with a smaller batch size.",
            max_query_batch_size.0
        ));
    }

    read_state(|state| {
        accounts
            .iter()
            .map(|account| state.data.tokens_balance_of(account))
            .collect()
    })
}

#[query]
pub fn icrc7_tokens(
    prev: core_nft_api::queries::icrc7::TokensArgs0,
    take: core_nft_api::queries::icrc7::TokensArgs1,
) -> core_nft_api::queries::icrc7::TokensResponse {
    if take.is_some() {
        let icrc7_max_take_value = read_state(|state| state.data.max_take_value.clone());
        let max_take_value = icrc7_max_take_value.unwrap_or(Nat::from(
            core_nft_common::types::icrc7::DEFAULT_MAX_TAKE_VALUE,
        ));

        if take.clone().unwrap().0 > max_take_value.0 {
            ic_cdk::trap(format!(
                "max_take_value exceeded. Limit is {}. Retry with a smaller take value.",
                max_take_value.0
            ));
        }
    }

    read_state(|state| {
        let prev = prev.unwrap_or(Nat::from(0 as u64));
        let take: usize = usize::try_from(
            take.unwrap_or_else(|| {
                state
                    .data
                    .default_take_value
                    .clone()
                    .unwrap_or(Nat::from(core_nft_common::types::icrc7::DEFAULT_TAKE_VALUE))
            })
            .0,
        )
        .unwrap_or(core_nft_common::types::icrc7::DEFAULT_TAKE_VALUE);

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
pub fn icrc7_tokens_of(
    account: core_nft_api::queries::icrc7::TokensOfArgs0,
    prev: core_nft_api::queries::icrc7::TokensOfArgs1,
    take: core_nft_api::queries::icrc7::TokensOfArgs2,
) -> core_nft_api::queries::icrc7::TokensOfResponse {
    if take.is_some() {
        let icrc7_max_take_value = read_state(|state| state.data.max_take_value.clone());
        let max_take_value = icrc7_max_take_value.unwrap_or(Nat::from(
            core_nft_common::types::icrc7::DEFAULT_MAX_TAKE_VALUE,
        ));

        if take.clone().unwrap().0 > max_take_value.0 {
            ic_cdk::trap(format!(
                "max_take_value exceeded. Limit is {}. Retry with a smaller take value.",
                max_take_value.0
            ));
        }
    }

    read_state(|state| {
        let prev = prev.unwrap_or(Nat::from(0 as u64));
        let take: usize = usize::try_from(
            take.unwrap_or_else(|| {
                state
                    .data
                    .default_take_value
                    .clone()
                    .unwrap_or(Nat::from(core_nft_common::types::icrc7::DEFAULT_TAKE_VALUE))
            })
            .0,
        )
        .unwrap_or(core_nft_common::types::icrc7::DEFAULT_TAKE_VALUE);

        let mut tokens: Vec<Nat> = state.data.tokens_ids_of_account(&account);
        tokens.sort();
        let start_index = tokens
            .iter()
            .position(|id| id > &prev)
            .unwrap_or(tokens.len());
        tokens.into_iter().skip(start_index).take(take).collect()
    })
}
