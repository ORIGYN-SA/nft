use candid::Nat;
use ic_asset_certification::Asset;
use storage_api_canister::cancel_upload;
use storage_api_canister::delete_file;
use storage_api_canister::finalize_upload;
use storage_api_canister::init_upload;
use storage_api_canister::store_chunk;
use tracing::info;
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use super::http;
use super::http::{certify_asset, uncertify_asset};
use crate::memory::get_data_storage_memory;
use crate::memory::VM;
use crate::state::read_state;
use crate::utils::trace;
use hex;
use ic_cdk::api::stable::{stable_size, WASM_PAGE_SIZE_IN_BYTES};
use ic_cdk::trap;
use ic_stable_structures::StableBTreeMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use storage_api_canister::types::value_custom::CustomValue as Value;
use storage_api_canister::utils;

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
    max_storage_size_wasm32: u128,
}

fn init_storage() -> StableBTreeMap<String, Value, VM> {
    let memory = get_data_storage_memory();
    StableBTreeMap::init(memory)
}

fn init_storage_raw() -> StableBTreeMap<String, Vec<u8>, VM> {
    let memory = get_data_storage_memory();
    StableBTreeMap::init(memory)
}

impl StorageData {
    pub fn new(max_storage_size_wasm32: u128) -> Self {
        Self {
            storage: init_storage(),
            storage_raw: init_storage_raw(),
            storage_raw_internal_metadata: HashMap::new(),
            max_storage_size_wasm32: max_storage_size_wasm32,
        }
    }

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

        if self.get_free_storage_size_bytes() < data_size {
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

        if self.get_free_storage_size_bytes() < data_size {
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

    pub fn get_free_storage_size_bytes(&self) -> u128 {
        let current_size = self.get_storage_size_bytes();
        if current_size >= self.max_storage_size_wasm32 {
            return 0;
        }
        let free_storage_size = self.max_storage_size_wasm32 - current_size;
        trace(&format!(
            "get_free_storage_size_bytes: {:?}",
            free_storage_size
        ));
        free_storage_size
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
        data: init_upload::Args,
    ) -> Result<init_upload::InitUploadResp, String> {
        // Check if the file already exists
        if self
            .storage_raw_internal_metadata
            .contains_key(&data.file_path)
        {
            return Err("File already exists".to_string());
        }

        if self.get_free_storage_size_bytes() < (data.file_size as u128) {
            return Err("Not enough storage".to_string());
        }
        let chunk_size = data.chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE);

        if chunk_size > DEFAULT_CHUNK_SIZE || chunk_size < 1 {
            return Err("Invalid chunk size, max size is 1Mb".to_string());
        }

        let num_chunks = (data.file_size + chunk_size - 1) / chunk_size;

        let metadata: InternalRawStorageMetadata = InternalRawStorageMetadata {
            file_path: data.file_path.clone(),
            file_hash: data.file_hash,
            file_size: data.file_size,
            received_size: 0,
            chunks_size: chunk_size,
            chunks: vec![vec![]; num_chunks as usize],
            state: UploadState::Init,
        };

        self.storage_raw_internal_metadata
            .insert(data.file_path.clone(), metadata);

        Ok(init_upload::InitUploadResp {})
    }

    pub fn store_chunk(
        &mut self,
        data: store_chunk::Args,
    ) -> Result<store_chunk::StoreChunkResp, String> {
        trace(&format!("store_chunk - hash_id: {:?}", data.file_path));

        let metadata = self
            .storage_raw_internal_metadata
            .get_mut(&data.file_path.clone())
            .ok_or("Upload not initialized".to_string())?;

        match metadata.state {
            UploadState::Init => {
                metadata.state = UploadState::InProgress;
            }
            UploadState::InProgress => (),
            UploadState::Finalized => {
                return Err("Storage - store_chunk - Upload already finalized".to_string());
            }
        }

        let file_size = metadata.file_size;
        let received_size = metadata.received_size;
        let chunk_index = usize::try_from(data.chunk_id.0).unwrap();

        if received_size + (data.chunk_data.len() as u64) > file_size {
            return Err("Chunk exceeds file size".to_string());
        }

        // Check if the chunk has already been stored
        if !metadata.chunks[chunk_index].is_empty() {
            return Err("Chunk already stored".to_string());
        }

        metadata.chunks[chunk_index] = data.chunk_data.clone();
        metadata.received_size = received_size + (data.chunk_data.len() as u64);

        Ok(store_chunk::StoreChunkResp {})
    }

    pub fn finalize_upload(
        &mut self,
        data: finalize_upload::Args,
    ) -> Result<finalize_upload::FinalizeUploadResp, String> {
        trace(&format!("finalize_upload - hash_id: {:?}", data.file_path));

        let mut metadata = self
            .storage_raw_internal_metadata
            .remove(&data.file_path.clone())
            .ok_or("Storage - finalize_upload - Upload not initialized".to_string())?;

        match metadata.state {
            UploadState::Init => {
                self.storage_raw_internal_metadata
                    .insert(data.file_path.clone(), metadata);
                return Err("Storage - finalize_upload - Upload not started".to_string());
            }
            UploadState::InProgress => {}
            UploadState::Finalized => {
                return Err("Storage - finalize_upload - Upload already finalized".to_string());
            }
        }

        let file_size = metadata.file_size as u128;
        let received_size = metadata.received_size as u128;

        if received_size != file_size {
            return Err(
                "Storage - finalize_upload - Incomplete upload. Upload failed, try again."
                    .to_string(),
            );
        }

        let mut file_data = Vec::with_capacity(file_size as usize);
        for chunk in metadata.chunks.clone() {
            file_data.extend(chunk);
        }

        if (file_data.len() as u64) != metadata.file_size {
            return Err(
                "Storage - finalize_upload - File size mismatch. Upload failed, try again."
                    .to_string(),
            );
        }

        let mut hasher = Sha256::new();
        hasher.update(&file_data);
        let calculated_hash = hex::encode(hasher.finalize());

        if calculated_hash != metadata.file_hash {
            return Err(
                "Storage - finalize_upload - File hash mismatch. Upload failed, try again."
                    .to_string(),
            );
        }

        metadata.chunks.clear();
        metadata.state = UploadState::Finalized;

        self.storage_raw_internal_metadata
            .insert(data.file_path.clone(), metadata.clone());
        self.storage_raw.insert(data.file_path, file_data.clone());
        // certify_asset(vec![Asset::new(metadata.file_path, file_data)]);

        Ok(finalize_upload::FinalizeUploadResp {})
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

    pub fn cancel_upload(
        &mut self,
        file_path: String,
    ) -> Result<cancel_upload::CancelUploadResp, String> {
        let metadata = self
            .storage_raw_internal_metadata
            .remove(&file_path)
            .ok_or("Upload not initialized".to_string())?;

        if metadata.state == UploadState::Finalized {
            trap("Cannot cancel a finalized upload");
        }

        Ok(cancel_upload::CancelUploadResp {})
    }

    pub fn delete_file(
        &mut self,
        file_path: String,
    ) -> Result<delete_file::DeleteFileResp, String> {
        let metadata = self
            .storage_raw_internal_metadata
            .remove(&file_path)
            .ok_or("File not found".to_string())?;

        if metadata.state != UploadState::Finalized {
            trap("Cannot delete a file that is not finalized");
        }

        self.storage_raw.remove(&file_path);
        uncertify_asset(vec![Asset::new(metadata.file_path, vec![])]);

        Ok(delete_file::DeleteFileResp {})
    }

    pub fn cache_miss(&self, path: String) -> Result<(), String> {
        trace(&format!("cache_miss: {:?}", path));

        let free_heap_size = self.get_free_heap_size_bytes();

        let metadata = self
            .storage_raw_internal_metadata
            .get(&path.clone())
            .ok_or("Upload not initialized".to_string())?;

        if metadata.state != UploadState::Finalized {
            trace(&format!(
                "This case should never happened ! skipping non-finalized file: {:?}",
                path
            ));

            return Err("Upload not finalized".to_string());
        }

        let file_size = metadata.file_size as u64;

        if free_heap_size < file_size {
            trace(&format!(
                "not enough storage, need to free cache : {:?} bytes requested",
                file_size - free_heap_size
            ));
            self.free_http_cache(file_size - free_heap_size)?;
        }

        let file_data = self.storage_raw.get(&path).unwrap();

        trace(&format!("certify_asset metadata : {:?}", metadata));

        certify_asset(vec![Asset::new(path.clone(), file_data)]);

        Ok(())
    }

    fn free_http_cache(&self, requested_size: u64) -> Result<(), String> {
        trace(&format!("free_http_cache: {:?}", requested_size));

        let mut freed_size = 0;

        for (key, metadata) in &self.storage_raw_internal_metadata {
            if freed_size >= requested_size {
                break;
            }

            if metadata.state != UploadState::Finalized {
                trace(&format!(
                    "This case should never happened ! skipping non-finalized file: {:?}.",
                    key
                ));

                continue;
            }

            let file_size = metadata.file_size as u64;

            let file_data = self.storage_raw.get(key).unwrap().clone();

            uncertify_asset(vec![Asset::new(
                metadata.file_path.clone(),
                file_data.clone(),
            )]);

            freed_size += file_size;
        }

        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get_free_heap_size_bytes(&self) -> u64 {
        let max_heap_size_wasm32 = 4 * 1024 * 1024 * 1024; // 4Gb
        let ret = max_heap_size_wasm32
            - (core::arch::wasm32::memory_size(0) as u64) * WASM_PAGE_SIZE_IN_BYTES; // 1Gb
        trace(&format!("get_free_heap_size_bytes: {:?}", ret));
        ret
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_free_heap_size_bytes(&self) -> u64 {
        let max_heap_size_wasm32 = 4 * 1024 * 1024 * 1024; // 4Gb
        let ret = max_heap_size_wasm32 - 3 * 1024 * 1024 * 1024; // 1Gb
        trace(&format!("get_free_heap_size_bytes: {:?}", ret));
        ret
    }
}
