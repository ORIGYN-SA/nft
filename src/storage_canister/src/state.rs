use candid::{CandidType, Nat, Principal};
use canister_state_macros::canister_state;
use storage_api_canister::{cancel_upload, delete_file, finalize_upload, init_upload, store_chunk};
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use crate::types::storage;
use serde::{Deserialize, Serialize};
use storage_api_canister::types::value_custom::CustomValue as Value;
use types::BuildVersion;
use types::{Cycles, TimestampMillis};
use utils::env::{CanisterEnv, Environment};
use utils::memory::MemorySize;

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
    pub http_cache: HttpCache,
}

impl Data {
    #[allow(clippy::too_many_arguments)]
    pub fn new(authorized_principals: Vec<Principal>, max_storage_size_wasm32: u128) -> Self {
        Self {
            authorized_principals: authorized_principals.into_iter().collect(),
            storage: storage::StorageData::new(max_storage_size_wasm32),
            http_cache: HttpCache::default(),
        }
    }

    pub fn insert_data(
        &mut self,
        data: Value,
        data_id: String,
        nft_id: Option<Nat>,
    ) -> Result<String, String> {
        self.storage.insert_data(data, data_id, nft_id)
    }

    pub fn update_data(
        &mut self,
        hash_id: String,
        data: Value,
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

impl Data {
    pub fn init_upload(
        &mut self,
        data: init_upload::Args,
    ) -> Result<init_upload::InitUploadResp, String> {
        self.storage.init_upload(data)
    }

    pub fn store_chunk(
        &mut self,
        data: store_chunk::Args,
    ) -> Result<store_chunk::StoreChunkResp, String> {
        self.storage.store_chunk(data)
    }

    pub fn finalize_upload(
        &mut self,
        data: finalize_upload::Args,
    ) -> Result<finalize_upload::FinalizeUploadResp, String> {
        self.storage.finalize_upload(data)
    }

    pub fn cancel_upload(
        &mut self,
        media_hash_id: String,
    ) -> Result<cancel_upload::CancelUploadResp, String> {
        self.storage.cancel_upload(media_hash_id)
    }

    pub fn delete_file(
        &mut self,
        media_hash_id: String,
    ) -> Result<delete_file::DeleteFileResp, String> {
        self.storage.delete_file(media_hash_id)
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
#[derive(CandidType, Deserialize, Serialize)]
pub struct HttpCache {
    pub current_size: Nat,
}

impl Default for HttpCache {
    fn default() -> Self {
        Self {
            current_size: Nat::from(0 as u64),
        }
    }
}

#[cfg(test)]
mod tests {}
