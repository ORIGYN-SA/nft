use crate::state::InitApprovalsArg;
use crate::types::collection_metadata::CollectionMetadata;
use crate::types::collection_metadata::CollectionMetadata;
use candid::{CandidType, Nat, Principal};
use serde::{Deserialize, Serialize};
use types::BuildVersion;

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct InitArgs {
    test_mode: bool,
    version: BuildVersion,
    commit_hash: String,
    authorized_principals: Vec<Principal>,
    minting_authorities: Vec<Principal>,
    description: Option<String>,
    symbol: String,
    name: String,
    logo: Option<Vec<u8>>,
    supply_cap: Option<Nat>,
    max_query_batch_size: Option<Nat>,
    max_update_batch_size: Option<Nat>,
    max_take_value: Option<Nat>,
    default_take_value: Option<Nat>,
    max_memo_size: Option<Nat>,
    atomic_batch_transfers: Option<bool>,
    tx_window: Option<Nat>,
    permitted_drift: Option<Nat>,
    max_canister_storage_threshold: Option<Nat>,
    collection_metadata: CollectionMetadata,
    approval_init: Option<InitApprovalsArg>,
}

#[derive(CandidType, Deserialize, Serialize, Debug, Clone)]
pub struct InitApprovalsArg {
    pub max_approvals: Option<u16>,
    pub max_approvals_per_token_or_collection: Option<u16>,
    pub max_revoke_approvals: Option<u16>,
    pub settle_to_approvals: Option<u16>,
    pub collection_approval_requires_token: Option<bool>,
}
