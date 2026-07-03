use core_nft_common::PrivateContentSystem;
use core_nft_common::PublicContentSystem;

use crate::state::Data;

use crate::state::RuntimeState;

use self::types::state::RuntimeStateV0;

pub mod types;

impl From<RuntimeStateV0> for RuntimeState {
    fn from(old_state: RuntimeStateV0) -> Self {
        Self {
            env: old_state.env,
            data: Data {
                permissions: old_state.data.permissions,
                description: old_state.data.description,
                symbol: old_state.data.symbol,
                name: old_state.data.name,
                logo: old_state.data.logo,
                supply_cap: old_state.data.supply_cap,
                max_query_batch_size: old_state.data.max_query_batch_size,
                max_update_batch_size: old_state.data.max_update_batch_size,
                max_take_value: old_state.data.max_take_value,
                default_take_value: old_state.data.default_take_value,
                max_memo_size: old_state.data.max_memo_size,
                atomic_batch_transfers: old_state.data.atomic_batch_transfers,
                tx_window: old_state.data.tx_window,
                permitted_drift: old_state.data.permitted_drift,
                max_canister_storage_threshold: old_state.data.max_canister_storage_threshold,
                tokens_list: old_state.data.tokens_list,
                tokens_list_by_owner: old_state.data.tokens_list_by_owner,
                private_content_system: PrivateContentSystem::default(),
                public_content_system: PublicContentSystem::default(),
                approval_init: old_state.data.approval_init,
                sub_canister_manager: old_state.data.sub_canister_manager,
                last_token_id: old_state.data.last_token_id,
                media_redirections: old_state.data.media_redirections,
                base_url: Default::default(),
            },
            principal_guards: old_state.principal_guards,
            sliding_window_guards: old_state.sliding_window_guards,
            internal_filestorage: old_state.internal_filestorage,
        }
    }
}
