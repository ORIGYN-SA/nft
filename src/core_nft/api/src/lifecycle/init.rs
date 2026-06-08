use bity_ic_types::BuildVersion;
use candid::{CandidType, Nat};
use core_nft_common::types::permissions::PermissionManager;
use core_nft_common::types::value_custom::CustomValue as Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
pub struct InitApprovalsArg {
    pub max_approvals_per_token_or_collection: Option<Nat>,
    pub max_revoke_approvals: Option<Nat>,
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct InitArgs {
    pub test_mode: bool,
    pub version: BuildVersion,
    pub commit_hash: String,
    pub permissions: PermissionManager,
    pub description: Option<String>,
    pub symbol: String,
    pub name: String,
    pub logo: Option<String>,
    pub supply_cap: Option<Nat>,
    pub max_query_batch_size: Option<Nat>,
    pub max_update_batch_size: Option<Nat>,
    pub max_take_value: Option<Nat>,
    pub default_take_value: Option<Nat>,
    pub max_memo_size: Option<Nat>,
    pub atomic_batch_transfers: Option<bool>,
    pub tx_window: Option<Nat>,
    pub permitted_drift: Option<Nat>,
    pub max_canister_storage_threshold: Option<Nat>,
    pub collection_metadata: HashMap<String, Value>,
    pub approval_init: InitApprovalsArg,
    pub base_url: Option<String>,
}
