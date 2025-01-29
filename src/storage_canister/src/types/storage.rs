use candid::{ Nat, CandidType };
use serde::{ Deserialize, Serialize };
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use ic_stable_structures::StableBTreeMap;
use crate::memory::get_data_storage_memory;
use crate::memory::VM;
use crate::utils;
use hex;
use sha2::{ Sha256, Digest };

#[derive(Serialize, Deserialize, CandidType)]
pub struct StorageData {
    storage: StableBTreeMap<String, Value, VM>,
    remaining_storage: usize,
    storage_capacity: usize,
    storage_used: usize,
}

const MAX_SWAP_INFO_BYTES_SIZE: usize = 250_000_000_000;

impl Default for StorageData {
    fn default() -> Self {
        let memory = get_data_storage_memory();

        Self {
            storage: StableBTreeMap::init(memory),
            remaining_storage: MAX_SWAP_INFO_BYTES_SIZE,
            storage_capacity: MAX_SWAP_INFO_BYTES_SIZE,
            storage_used: 0,
        }
    }
}

impl StorageData {
    pub fn get_data(&self, hash_id: String) -> Result<Value, String> {
        self.storage.get(&hash_id).cloned().ok_or("Data not found".to_string())
    }

    pub fn remove_data(&mut self, hash_id: String) -> Result<Value, String> {
        let data = self.storage.remove(&hash_id).ok_or("Data not found".to_string())?;
        let data_size: usize = utils::get_value_size(data);

        self.remaining_storage += data_size;
        self.storage_used -= data_size;

        Ok(data)
    }

    pub fn update_data(&mut self, hash_id: String, data: Value) -> Result<(String, Value), String> {
        let data_size: usize = utils::get_value_size(data);

        if self.remaining_storage < data_size {
            return Err(
                "Not enough storage. You should remove this, and store again in another instance of storage canister.".to_string()
            );
        }

        let previous_data_value = self.storage.get(&hash_id).cloned();
        let previous_data_size: usize = utils::get_value_size(data);
        self.remaining_storage += previous_data_size;
        self.storage_used -= previous_data_size;

        self.storage.insert(hash_id.clone(), data);
        self.remaining_storage -= data_size;
        self.storage_used += data_size;

        Ok((hash_id, previous_data_value))
    }

    pub fn insert_data(
        &mut self,
        data: Value,
        data_id: Nat,
        nft_id: Nat
    ) -> Result<String, String> {
        let data_size: usize = utils::get_value_size(data);

        if self.remaining_storage < data_size {
            return Err("Not enough storage".to_string());
        }

        let hash_id: String = self
            .hash_data(data_id, nft_id)
            .map_err(|e| format!("Error hashing data: {}", e))?;

        self.storage.insert(hash_id, data);
        self.remaining_storage -= data_size;
        self.storage_used += data_size;

        Ok(hash_id)
    }

    fn hash_data(&self, data_id: Nat, nft_id: Nat) -> Result<String, String> {
        let mut hasher = Sha256::new();
        hasher.update(data_id.to_string().as_bytes());
        hasher.update(nft_id.to_string().as_bytes());
        let result = hasher.finalize();

        let hash_string = hex::encode(result);
        Ok(hash_string)
    }
}
