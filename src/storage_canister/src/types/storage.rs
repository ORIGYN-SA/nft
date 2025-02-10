use candid::Nat;
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use crate::memory::get_data_storage_memory;
use crate::memory::VM;
use hex;
use ic_cdk::api::stable::{stable_size, WASM_PAGE_SIZE_IN_BYTES};
use ic_stable_structures::StableBTreeMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use storage_api_canister::types::value_custom::CustomValue as Value;
use storage_api_canister::utils;

#[derive(Serialize, Deserialize)]
pub struct StorageData {
    #[serde(skip, default = "init_storage")]
    storage: StableBTreeMap<String, Value, VM>,
}

fn init_storage() -> StableBTreeMap<String, Value, VM> {
    let memory = get_data_storage_memory();
    StableBTreeMap::init(memory)
}

impl Default for StorageData {
    fn default() -> Self {
        Self {
            storage: init_storage(),
        }
    }
}

impl StorageData {
    pub fn get_data(&self, hash_id: String) -> Result<Value, String> {
        self.storage
            .get(&hash_id)
            .map(|v| v.clone())
            .ok_or("Data not found".to_string())
    }

    pub fn remove_data(&mut self, hash_id: String) -> Result<Value, String> {
        let data = self
            .storage
            .remove(&hash_id)
            .ok_or("Data not found".to_string())?;

        Ok(data)
    }

    pub fn update_data(
        &mut self,
        hash_id: String,
        data: Value,
    ) -> Result<(String, Option<Value>), String> {
        let data_size: u128 = utils::get_value_size(data.clone());

        if self.get_storage_size_bytes() < data_size {
            return Err(
                "Not enough storage. You should remove this, and store again in another instance of storage canister.".to_string()
            );
        }

        let previous_data_value = self.storage.get(&hash_id).map(|v| v.clone());
        self.storage.insert(hash_id.clone(), data);

        Ok((hash_id, previous_data_value))
    }

    pub fn insert_data(
        &mut self,
        data: Value,
        data_id: String,
        nft_id: Option<Nat>,
    ) -> Result<String, String> {
        let data_size: u128 = utils::get_value_size(data.clone());

        if self.get_storage_size_bytes() < data_size {
            return Err("Not enough storage".to_string());
        }

        let hash_id: String = self
            .hash_data(data_id, nft_id)
            .map_err(|e| format!("Error hashing data: {}", e))?;

        self.storage.insert(hash_id.clone(), data);

        Ok(hash_id)
    }

    pub fn get_storage_size_bytes(&self) -> u128 {
        let num_pages = stable_size();
        let bytes = (num_pages as usize) * (WASM_PAGE_SIZE_IN_BYTES as usize);
        bytes as u128
    }

    fn hash_data(&self, data_id: String, nft_id: Option<Nat>) -> Result<String, String> {
        let mut hasher = Sha256::new();
        hasher.update(data_id.as_bytes());
        match nft_id {
            Some(nft_id) => hasher.update(nft_id.to_string().as_bytes()),
            None => (),
        }
        let result = hasher.finalize();

        let hash_string = hex::encode(result);
        Ok(hash_string)
    }
}
