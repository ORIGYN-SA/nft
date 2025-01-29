use std::collections::HashMap;

use candid::{ CandidType, Nat, Principal };
use canister_state_macros::canister_state;
use serde::{ Deserialize, Serialize };
use types::BuildVersion;
use types::{ Cycles, TimestampMillis };
use utils::env::{ CanisterEnv, Environment };
use utils::memory::MemorySize;
use crate::types::collection_metadata::CollectionMetadata;
use crate::types::nft::Icrc7Token;
use icrc_ledger_types::icrc1::account::Account;

canister_state!(RuntimeState);

#[derive(Serialize, Deserialize)]
pub struct RuntimeState {
    pub env: CanisterEnv,
    pub data: Data,
}

impl RuntimeState {
    pub fn new(env: CanisterEnv, data: Data) -> Self {
        RuntimeState { env, data }
    }

    pub fn is_caller_governance_principal(&self) -> bool {
        self.data.authorized_principals.contains(&self.env.caller())
    }

    pub fn metrics(&self) -> Metrics {
        Metrics {
            canister_info: CanisterInfo {
                test_mode: self.env.is_test_mode(),
                now: self.env.now(),
                version: self.env.version(),
                commit_hash: self.env.commit_hash().to_string(),
                memory_used: MemorySize::used(),
                cycles_balance: self.env.cycles_balance(),
            },
            authorized_principals: self.data.authorized_principals.to_vec(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Data {
    pub authorized_principals: Vec<Principal>,
    pub minting_authorities: Vec<Principal>,
    pub description: Option<String>,
    pub symbol: String,
    pub name: String,
    pub logo: Option<Vec<u8>>,
    pub supply_cap: Option<Nat>,
    pub max_query_batch_size: Option<Nat>,
    pub max_update_batch_size: Option<Nat>,
    pub max_take_value: Option<Nat>,
    pub default_take_value: Option<Nat>,
    pub max_memo_size: Option<Nat>,
    pub atomic_batch_transfers: Option<bool>,
    pub tx_window: Option<Nat>,
    pub permitted_drift: Option<Nat>,
    pub collection_metadata: CollectionMetadata,
    pub tokens_list: HashMap<Nat, Icrc7Token>,
    pub approval_init: Option<InitApprovalsArg>,
    // pub archive_init: Option<InitArchiveArg>,
}

impl Data {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
        collection_metadata: CollectionMetadata,
        approval_init: Option<InitApprovalsArg>
    ) -> Self {
        Self {
            authorized_principals: authorized_principals.into_iter().collect(),
            minting_authorities: minting_authorities.into_iter().collect(),
            description,
            symbol,
            name,
            logo,
            supply_cap,
            max_query_batch_size,
            max_update_batch_size,
            max_take_value,
            default_take_value,
            max_memo_size,
            atomic_batch_transfers,
            tx_window,
            permitted_drift,
            collection_metadata,
            tokens_list: HashMap::new(),
            approval_init,
        }
    }

    pub fn get_token_by_id(&self, token_id: &Nat) -> Option<&Icrc7Token> {
        self.tokens_list.get(token_id)
    }

    pub fn update_token_by_id(&mut self, token_id: &Nat, token: &Icrc7Token) {
        self.tokens_list.insert(token_id.clone(), token.clone());
    }

    pub fn add_token(&mut self, token: &Icrc7Token) {
        self.tokens_list.insert(token.clone().token_id, token.clone());
    }

    pub fn owner_of(&self, token_id: &Nat) -> Option<Account> {
        self.tokens_list.get(token_id).map(|token| token.token_owner.clone())
    }

    pub fn tokens_balance_of(&self, owner: &Account) -> Nat {
        let count = self.tokens_list
            .values()
            .filter(|token| &token.token_owner == owner)
            .count() as u64;

        Nat::from(count)
    }

    pub fn tokens_of_account(&self, owner: &Account) -> Vec<Icrc7Token> {
        self.tokens_list
            .values()
            .filter(|token| &token.token_owner == owner)
            .cloned()
            .collect()
    }

    pub fn tokens_ids_of_account(&self, owner: &Account) -> Vec<Nat> {
        self.tokens_list
            .iter()
            .filter(|(_, token)| &token.token_owner == owner)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn total_supply(&self) -> Nat {
        Nat::from(self.tokens_list.len() as u64)
    }
}

#[derive(CandidType, Deserialize, Serialize, Debug)]
pub struct InitApprovalsArg {
    pub max_approvals: Option<u16>,
    pub max_approvals_per_token_or_collection: Option<u16>,
    pub max_revoke_approvals: Option<u16>,
    pub settle_to_approvals: Option<u16>,
    pub collection_approval_requires_token: Option<bool>,
}

#[derive(CandidType, Deserialize, Serialize, Debug)]
pub struct StorageCanisterInfo {
    pub canister_id: Principal,
    pub canister_stable_memory_used: Nat,
    pub canister_stable_memory_free: Nat,
    pub cycles_balance: Cycles,
    pub cycles_consummed: Cycles,
}

#[derive(CandidType, Deserialize, Serialize, Debug)]
pub struct StorageCanisterConfig {
    pub canister_ids: Vec<StorageCanisterInfo>,
    pub max_canister_storage: Option<u128>,
    pub max_canister_cycles: Option<u128>,
}

#[derive(CandidType, Serialize)]
pub struct Metrics {
    pub canister_info: CanisterInfo,
    pub authorized_principals: Vec<Principal>,
}

#[derive(CandidType, Deserialize, Serialize)]
pub struct CanisterInfo {
    pub now: TimestampMillis,
    pub test_mode: bool,
    pub version: BuildVersion,
    pub commit_hash: String,
    pub memory_used: MemorySize,
    pub cycles_balance: Cycles,
}

#[cfg(test)]
mod tests {}
