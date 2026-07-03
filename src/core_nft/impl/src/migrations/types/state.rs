use crate::state::InternalFilestorage;
use bity_ic_types::TimestampNanos;
use bity_ic_utils::env::CanisterEnv;
use candid::Principal;
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

#[derive(Serialize, Deserialize)]
pub struct RuntimeStateV0 {
    pub env: CanisterEnv,
    pub data: DataV0,
    pub principal_guards: BTreeSet<Principal>,
    pub sliding_window_guards: HashMap<candid::Nat, Vec<TimestampNanos>>, // per token id
    pub internal_filestorage: InternalFilestorage,
}

use crate::types::nft::Icrc7Token;
use crate::Nat;
use core_nft_api::init::InitApprovalsArg;
use core_nft_common::PermissionManager;
use core_nft_common::StorageSubCanisterManager;
#[derive(Serialize, Deserialize)]
pub struct DataV0 {
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
    pub tokens_list: HashMap<Nat, Icrc7Token>,
    pub tokens_list_by_owner: HashMap<Account, Vec<Nat>>,
    pub approval_init: InitApprovalsArg,
    pub sub_canister_manager: StorageSubCanisterManager,
    pub last_token_id: Nat,
    pub media_redirections: HashMap<String, String>,
}
