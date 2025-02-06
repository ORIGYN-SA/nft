use candid::{ CandidType, Nat, Principal };
use canister_state_macros::canister_state;
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use storage_api_canister::types::value_custom::CustomValue as Value;
use serde::{ Deserialize, Serialize };
use types::BuildVersion;
use types::{ Cycles, TimestampMillis };
use utils::env::{ CanisterEnv, Environment };
use utils::memory::MemorySize;
use crate::types::storage;

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
    pub storage: storage::StorageData,
}

impl Data {
    #[allow(clippy::too_many_arguments)]
    pub fn new(authorized_principals: Vec<Principal>) -> Self {
        Self {
            authorized_principals: authorized_principals.into_iter().collect(),
            storage: storage::StorageData::default(),
        }
    }

    pub fn insert_data(
        &mut self,
        data: Value,
        data_id: String,
        nft_id: Option<Nat>
    ) -> Result<String, String> {
        self.storage.insert_data(data, data_id, nft_id)
    }

    pub fn update_data(
        &mut self,
        hash_id: String,
        data: Value
    ) -> Result<(String, Option<Value>), String> {
        self.storage.update_data(hash_id, data)
    }

    pub fn remove_data(&mut self, hash_id: String) -> Result<Value, String> {
        self.storage.remove_data(hash_id)
    }

    pub fn get_data(&self, hash_id: String) -> Result<Value, String> {
        self.storage.get_data(hash_id)
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

#[cfg(test)]
mod tests {}
