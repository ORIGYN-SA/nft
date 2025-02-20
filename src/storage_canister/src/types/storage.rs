use candid::Nat;
use ic_asset_certification::Asset;
use ic_cdk::api::call::RejectionCode;
use storage_api_canister::finalize_upload;
use storage_api_canister::finalize_upload::FinalizeUploadResp;
use storage_api_canister::init_upload;
use storage_api_canister::store_chunk;
use tracing::info;
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use crate::memory::get_data_storage_memory;
use crate::memory::VM;
use hex;
use ic_cdk::api::stable::{ stable_size, WASM_PAGE_SIZE_IN_BYTES };
use ic_stable_structures::StableBTreeMap;
use serde::{ Deserialize, Serialize };
use sha2::{ Digest, Sha256 };
use storage_api_canister::types::value_custom::CustomValue as Value;
use storage_api_canister::utils;
use std::collections::HashMap;
use crate::utils::trace;

use super::http::certify_asset;

const DEFAULT_CHUNK_SIZE: u64 = 1 * 1024 * 1024;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum UploadState {
    Init,
    InProgress,
    Finalized,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InternalRawStorageMetadata {
    pub file_path: String,
    pub file_hash: String,
    pub file_size: u64,
    pub received_size: u64,
    pub chunks_size: u64,
    pub chunks: Vec<Vec<u8>>,
    pub state: UploadState,
}

#[derive(Serialize, Deserialize)]
pub struct StorageData {
    #[serde(skip, default = "init_storage")]
    storage: StableBTreeMap<String, Value, VM>,
    #[serde(skip, default = "init_storage_raw")]
    storage_raw: StableBTreeMap<String, Vec<u8>, VM>,
    storage_raw_internal_metadata: HashMap<String, InternalRawStorageMetadata>,
}

fn init_storage() -> StableBTreeMap<String, Value, VM> {
    let memory = get_data_storage_memory();
    StableBTreeMap::init(memory)
}

fn init_storage_raw() -> StableBTreeMap<String, Vec<u8>, VM> {
    let memory = get_data_storage_memory();
    StableBTreeMap::init(memory)
}

impl Default for StorageData {
    fn default() -> Self {
        Self {
            storage: init_storage(),
            storage_raw: init_storage_raw(),
            storage_raw_internal_metadata: HashMap::new(),
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
        let data = self.storage.remove(&hash_id).ok_or("Data not found".to_string())?;

        Ok(data)
    }

    pub fn update_data(
        &mut self,
        hash_id: String,
        data: Value
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
        nft_id: Option<Nat>
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

impl StorageData {
    pub fn init_upload(
        &mut self,
        data: init_upload::Args
    ) -> Result<init_upload::InitUploadResp, String> {
        info!("self.get_storage_size_bytes(): {:?}", self.get_storage_size_bytes());
        trace(
            &format!(
                "init_upload self.get_storage_size_bytes() {:?}",
                self.get_storage_size_bytes()
            )
        );
        if self.get_storage_size_bytes() < (data.file_size as u128) {
            return Err("Not enough storage".to_string());
        }
        let chunk_size = data.chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE);

        if chunk_size > DEFAULT_CHUNK_SIZE || chunk_size < 1 {
            return Err("Invalid chunk size, max size is 1Mb".to_string());
        }

        let num_chunks = (data.file_size + chunk_size - 1) / chunk_size;

        let metadata: InternalRawStorageMetadata = InternalRawStorageMetadata {
            file_path: data.file_path,
            file_hash: data.file_hash,
            file_size: data.file_size,
            received_size: 0,
            chunks_size: chunk_size,
            chunks: vec![vec![]; num_chunks as usize],
            state: UploadState::Init,
        };

        self.storage_raw_internal_metadata.insert(data.media_hash_id.clone(), metadata);

        Ok(init_upload::InitUploadResp {})
    }

    pub fn store_chunk(
        &mut self,
        data: store_chunk::Args
    ) -> Result<store_chunk::StoreChunkResp, String> {
        trace(&format!("store_chunk - hash_id: {:?}", data.media_hash_id));

        let metadata = self.storage_raw_internal_metadata
            .get_mut(&data.media_hash_id.clone())
            .ok_or("Upload not initialized".to_string())?;

        match metadata.state {
            UploadState::Init => {
                metadata.state = UploadState::InProgress;
            }
            UploadState::InProgress => (),
            UploadState::Finalized => {
                return Err("Upload already finalized".to_string());
            }
        }

        let file_size = metadata.file_size;
        let received_size = metadata.received_size;
        let chunk_index = usize::try_from(data.chunk_id.0).unwrap();

        if received_size + (data.chunk_data.len() as u64) > file_size {
            return Err("Chunk exceeds file size".to_string());
        }

        metadata.chunks[chunk_index] = data.chunk_data.clone();
        metadata.received_size = received_size + (data.chunk_data.len() as u64);

        Ok(store_chunk::StoreChunkResp {})
    }

    pub fn finalize_upload(
        &mut self,
        data: finalize_upload::Args
    ) -> Result<finalize_upload::FinalizeUploadResp, String> {
        trace(&format!("finalize_upload - hash_id: {:?}", data.media_hash_id));

        let mut metadata = self.storage_raw_internal_metadata
            .remove(&data.media_hash_id.clone())
            .ok_or("Upload not initialized".to_string())?;

        match metadata.state {
            UploadState::Init => {
                self.storage_raw_internal_metadata.insert(data.media_hash_id.clone(), metadata);
                return Err("Upload not started".to_string());
            }
            UploadState::InProgress => {}
            UploadState::Finalized => {
                return Err("Upload already finalized".to_string());
            }
        }

        let file_size = metadata.file_size as u128;
        let received_size = metadata.received_size as u128;

        if received_size != file_size {
            return Err("Incomplete upload. Upload failed, try again.".to_string());
        }

        let mut file_data = Vec::with_capacity(file_size as usize);
        for chunk in metadata.chunks.clone() {
            file_data.extend(chunk);
        }

        if (file_data.len() as u64) != metadata.file_size {
            return Err("File size mismatch. Upload failed, try again.".to_string());
        }

        let mut hasher = Sha256::new();
        hasher.update(&file_data);
        let calculated_hash = hex::encode(hasher.finalize());

        if calculated_hash != metadata.file_hash {
            return Err("File hash mismatch. Upload failed, try again.".to_string());
        }

        metadata.chunks.clear();
        metadata.state = UploadState::Finalized;

        self.storage_raw_internal_metadata.insert(data.media_hash_id.clone(), metadata.clone());
        self.storage_raw.insert(data.media_hash_id, file_data.clone());
        certify_asset(vec![Asset::new(metadata.file_path, file_data)]);

        Ok(finalize_upload::FinalizeUploadResp {})
    }

    pub fn get_raw_data(
        &self,
        media_hash_id: String
    ) -> Result<(InternalRawStorageMetadata, Vec<u8>), String> {
        trace(&format!("get_raw_data - hash_id: {:?}", media_hash_id));

        let metadata = self.storage_raw_internal_metadata
            .get(&media_hash_id)
            .ok_or("Data not found".to_string())?;

        trace(&format!("get_raw_data - metadata: {:?}", metadata));
        match metadata.state {
            UploadState::Finalized => {
                let raw_data = self.storage_raw
                    .get(&media_hash_id)
                    .map(|v| v.clone())
                    .ok_or("Data not found".to_string())?;
                Ok((metadata.clone(), raw_data))
            }
            _ => Err("Data not finalized".to_string()),
        }
    }

    pub fn get_all_files(&self) -> Vec<(InternalRawStorageMetadata, Vec<u8>)> {
        self.storage_raw_internal_metadata
            .iter()
            .filter_map(|(hash_id, metadata)| {
                if metadata.state == UploadState::Finalized {
                    let raw_data = self.storage_raw.get(hash_id).unwrap().clone();
                    Some((metadata.clone(), raw_data))
                } else {
                    None
                }
            })
            .collect()
    }
}
