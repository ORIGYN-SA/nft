use bity_ic_canister_state_macros::canister_state;
use bity_ic_types::{BuildVersion, Cycles, TimestampMillis, TimestampNanos};
use bity_ic_utils::env::{CanisterEnv, Environment};
use bity_ic_utils::memory::MemorySize;
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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

    pub fn is_caller_authorized(&self) -> bool {
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
            authorized_principals: self.data.authorized_principals.iter().cloned().collect(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Data {
    pub authorized_principals: HashSet<Principal>,
    pub ledger_canister_id: Principal,
    pub last_index_update: TimestampNanos,
    pub last_block_id: u64,
}

impl Data {
    #[allow(clippy::too_many_arguments)]
    pub fn new(authorized_principals: Vec<Principal>, ledger_canister_id: Principal) -> Self {
        Self {
            authorized_principals: authorized_principals.clone().into_iter().collect(),
            ledger_canister_id,
            last_index_update: ic_cdk::api::time(),
            last_block_id: 0,
        }
    }

    pub fn add_authorized_principals(&mut self, new_principals: Vec<Principal>) {
        for principal in new_principals {
            self.authorized_principals.insert(principal);
        }
    }

    pub fn remove_authorized_principals(&mut self, principals_to_remove: Vec<Principal>) {
        for principal in principals_to_remove {
            self.authorized_principals.remove(&principal);
        }
    }
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
