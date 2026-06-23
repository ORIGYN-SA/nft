use crate::finalize_upload::FinalizeUploadError;
use crate::types::cancel_upload::CancelUploadResp;
use crate::types::init_upload::InitUploadError;
use crate::types::init_upload::InitUploadResp;
use crate::types::store_chunk::StoreChunkError;
use crate::CHUNK_SIZE;
use crate::MAX_CONTENT_SIZE;
use bity_ic_storage_canister_api::storage::UploadState;
use candid::Nat;
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PublicContentStatus {
    PendingUpload,
    PendingMinting,
    Active,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum PublicContentError {
    FileAlreadyExists,
    InvalidStateTransition,
    NotEnabled,
    NotFound,
    Unauthorized,
    InvalidStatus,
    ContentTooLarge,
    InvalidChunk,
    ConcurrentManagementCall,
    StorageCanisterError(String),
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct PendingUpload {
    pub expected_chunks: usize,
    pub received_chunks: HashMap<Nat, Vec<u8>>,
    pub chunk_size: usize,
    pub timestamp_ns: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct EntryDetailResp {
    pub name: String,
    pub state: UploadState,
    pub hash: String,
    pub file_size: u64,
    pub storage_canister_id: Principal,
    pub storage_path: String,
}

// NOTE: I introduce here a lot of denormalizing to allow faster access to files needed
#[derive(Serialize, Deserialize, Clone)]
pub struct PublicContentSystem {
    pub nft_public: HashMap<Nat, NftPublicRecord>,
    pub premint_cache: HashMap<String, PublicEntry>, // NOTE: not hash, but file path
    pub file_to_nfts: HashMap<String, HashSet<Nat>>,
    pub nft_to_files: HashMap<Nat, HashSet<String>>,
    pub all_files_index: HashMap<String, PublicEntry>,
}

impl PublicContentSystem {
    pub fn get_nft_public_entry(
        &self,
        token_id: Nat,
        entry_name: &str,
    ) -> Result<PublicEntry, String> {
        self.nft_public
            .get(&token_id)
            .ok_or_else(|| format!("NFT {token_id} not found"))?
            .entries
            .get(entry_name)
            .cloned()
            .ok_or_else(|| format!("Public entry '{entry_name}' not found for NFT {token_id}"))
    }

    pub fn get_files_by_nft(&self, token_id: &Nat) -> Vec<String> {
        self.nft_to_files
            .get(token_id)
            .map(|files| files.iter().cloned().collect())
            .unwrap_or_default()
    }

    // NEW: Retrieve a file record (active or cached) by its unique storage file path
    pub fn get_public_file_by_path(&self, file_path: &str) -> Option<PublicEntry> {
        // First check the active records across minted NFTs
        if let Some(nfts) = self.file_to_nfts.get(file_path) {
            for token_id in nfts {
                if let Some(record) = self.nft_public.get(token_id) {
                    for entry in record.entries.values() {
                        if entry.storage_path == file_path {
                            return Some(entry.clone());
                        }
                    }
                }
            }
        }
        // Fallback to checking the premint cache structure
        self.premint_cache.get(file_path).cloned()
    }

    pub fn remove_entry_by_path(&mut self, file_path: &String) -> Result<(), PublicContentError> {
        // Guard: If it's already active/linked to an NFT, it cannot be raw-deleted from here
        if self.file_to_nfts.contains_key(file_path) {
            return Err(PublicContentError::InvalidStateTransition);
        }

        // Remove from the cache map
        match self.premint_cache.remove(file_path) {
            Some(_) => Ok(()),
            None => Err(PublicContentError::NotFound),
        }
    }

    // MODIFIED: Uses file_path string key instead of hash
    pub fn init_premint_validate(
        &self,
        file_path: &String,
        file_size: u64,
    ) -> crate::types::init_upload::Response {
        if self.premint_cache.contains_key(file_path) {
            return Err(InitUploadError::FileAlreadyExists);
        }

        if file_size > MAX_CONTENT_SIZE {
            return Err(InitUploadError::ContentTooLarge);
        }

        Ok(InitUploadResp {})
    }

    // MODIFIED: Uses file_path string key instead of hash
    pub fn init_premint_store(
        &mut self,
        file_path: String,
        storage_canister_id: Principal,
        storage_path: String,
        expected_chunks: usize,
        file_size: u64,
    ) -> crate::types::init_upload::Response {
        if self.premint_cache.contains_key(&file_path) {
            return Err(InitUploadError::FileAlreadyExists);
        }

        let entry = PublicEntry {
            state: UploadState::Init,
            hash: file_path.clone(), // Using path as the primary cache identity tracker
            file_size,
            storage_canister_id,
            storage_path,
            pending_upload: Some(PendingUpload {
                expected_chunks,
                received_chunks: HashMap::new(),
                chunk_size: CHUNK_SIZE,
                timestamp_ns: ic_cdk::api::time(),
            }),
            format_version: 1,
            created_at_ns: ic_cdk::api::time(),
        };

        self.premint_cache.insert(file_path, entry);
        Ok(InitUploadResp {})
    }

    pub fn upload_chunk(
        &mut self,
        file_path: &String,
        chunk_index: Nat,
        data: Vec<u8>,
    ) -> Result<(), StoreChunkError> {
        let entry = self
            .premint_cache
            .get_mut(file_path)
            .ok_or(StoreChunkError::InvalidFilePath)?;

        if entry.state != UploadState::Init {
            // or UploadState::PendingUpload
            return Err(StoreChunkError::InvalidStateTransition);
        }

        let pending = entry
            .pending_upload
            .as_mut()
            .ok_or(StoreChunkError::InvalidStateTransition)?;

        // Custom Nat comparison fix
        if chunk_index >= Nat::from(pending.expected_chunks) {
            return Err(StoreChunkError::InvalidChunkId);
        }

        if data.len() > CHUNK_SIZE {
            return Err(StoreChunkError::InvalidChunkData);
        }

        pending.received_chunks.insert(chunk_index, data);
        Ok(())
    }

    // MODIFIED: Uses file_path string key instead of hash
    pub fn finalize_upload(&mut self, file_path: &String) -> Result<(), FinalizeUploadError> {
        let entry = self
            .premint_cache
            .get_mut(file_path)
            .ok_or(FinalizeUploadError::UploadNotStarted)?;

        if entry.state != UploadState::InProgress {
            return Err(FinalizeUploadError::InvalidStateTransition);
        }

        let pending = entry
            .pending_upload
            .as_ref()
            .ok_or(FinalizeUploadError::InvalidStateTransition)?;

        if pending.received_chunks.len() != pending.expected_chunks {
            return Err(FinalizeUploadError::IncompleteUpload);
        }

        let actual_size: u64 = pending
            .received_chunks
            .values()
            .map(|c| c.len() as u64)
            .sum();

        if actual_size != entry.file_size {
            return Err(FinalizeUploadError::FileSizeMismatch);
        }

        entry.state = UploadState::Finalized;
        entry.pending_upload = None;

        Ok(())
    }

    // MODIFIED: Uses file_path string key instead of hash
    pub fn cancel_upload(&mut self, file_path: &String) -> crate::cancel_upload::Response {
        self.premint_cache.remove(file_path);
        Ok(CancelUploadResp {})
    }

    // MODIFIED: Cross-references are resolved cleanly against the file_path cache key mappings
    pub fn mint_public_content(
        &mut self,
        entries_to_mint_paths: HashMap<String, String>,
        token_id: Nat,
    ) -> Result<(), PublicContentError> {
        if self.nft_public.contains_key(&token_id) {
            return Err(PublicContentError::FileAlreadyExists);
        }

        let mut public_record = NftPublicRecord::new();

        for (entry_name, file_path) in entries_to_mint_paths {
            let mut entry = match self.premint_cache.remove(&file_path) {
                Some(cached_entry) => cached_entry,
                None => self
                    .nft_public
                    .values()
                    .find_map(|record| record.entries.values().find(|e| e.hash == file_path))
                    .cloned()
                    .ok_or(PublicContentError::NotFound)?,
            };

            if entry.state != UploadState::Finalized {
                return Err(PublicContentError::InvalidStateTransition);
            }

            entry.state = UploadState::Finalized;

            let file_identity = entry.storage_path.clone();

            self.file_to_nfts
                .entry(file_identity.clone())
                .or_default()
                .insert(token_id.clone());

            self.nft_to_files
                .entry(token_id.clone())
                .or_default()
                .insert(file_identity);

            public_record.entries.insert(entry_name, entry);
        }

        self.nft_public.insert(token_id, public_record);
        Ok(())
    }

    pub fn burn_public_content(&mut self, token_id: &Nat) -> Vec<String> {
        let mut files_to_delete = Vec::new();

        if let Some(record) = self.nft_public.get_mut(token_id) {
            record.burned_at_ns = Some(ic_cdk::api::time());
        }

        if let Some(associated_files) = self.nft_to_files.remove(token_id) {
            for file_id in associated_files {
                if let Some(nft_set) = self.file_to_nfts.get_mut(&file_id) {
                    nft_set.remove(token_id);

                    if nft_set.is_empty() {
                        self.file_to_nfts.remove(&file_id);
                        files_to_delete.push(file_id);
                    }
                }
            }
        }

        files_to_delete
    }

    pub fn collect_expired_burned(
        &self,
        now_ns: u64,
        threshold_ns: u64,
    ) -> Vec<(Nat, Vec<(String, candid::Principal, String)>)> {
        self.nft_public
            .iter()
            .filter_map(|(token_id, record)| {
                record.burned_at_ns.and_then(|burned_at| {
                    if burned_at + threshold_ns <= now_ns {
                        let files: Vec<_> = record
                            .entries
                            .iter()
                            .map(|(name, entry)| {
                                (
                                    name.clone(),
                                    entry.storage_canister_id,
                                    entry.storage_path.clone(),
                                )
                            })
                            .collect();
                        Some((token_id.clone(), files))
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    pub fn remove_burned_record(&mut self, token_id: &Nat) {
        self.nft_public.remove(token_id);
    }

    pub fn get_paginated_uploads(
        &self,
        prev: Option<Nat>,
        take: Option<Nat>,
    ) -> HashMap<String, UploadState> {
        let start = usize::try_from(prev.unwrap_or(Nat::from(0u64)).0).unwrap_or(0);
        let count = usize::try_from(take.unwrap_or(Nat::from(100u64)).0).unwrap_or(100);

        // 1. Create an iterator over active NFT entries
        // We flatten the nested HashMap<Nat, NftPublicRecord> -> HashMap<String, PublicEntry>
        let nft_iter = self.nft_public.values().flat_map(|record| {
            record.entries.iter().map(|(name, entry)| {
                // Use storage_path as the unique key for consistency with premint_cache
                (entry.storage_path.clone(), entry.state.clone())
            })
        });

        // 2. Create an iterator over premint cache entries
        let cache_iter = self
            .premint_cache
            .iter()
            .map(|(path, entry)| (path.clone(), entry.state.clone()));

        // 3. Chain them together
        // Note: If stable ordering across upgrades is critical, you MUST sort here.
        // Sorting requires collecting into a Vec first, which trades memory for stability.
        // If stability isn't critical, this direct chain is most efficient.

        let mut combined: Vec<(String, UploadState)> = nft_iter.chain(cache_iter).collect();

        // Optional: Sort by path to ensure deterministic pagination
        combined.sort_by(|a, b| a.0.cmp(&b.0));

        combined.into_iter().skip(start).take(count).collect()
    }

    pub fn get_all_files(&self) -> HashMap<String, PublicEntry> {
        let mut all_files = HashMap::new();

        // 1. Process Active/Minted Files
        // Iterate over file_to_nfts which contains every unique file_path currently in use.
        for (file_path, nft_set) in &self.file_to_nfts {
            // We only need one token_id to retrieve the entry data.
            // Use .next() to get an Option<&Nat>, then use that reference for lookup.
            if let Some(token_id_ref) = nft_set.iter().next() {
                if let Some(record) = self.nft_public.get(token_id_ref) {
                    // Find the specific entry in the NFT record that matches this file_path
                    if let Some(entry) = record
                        .entries
                        .values()
                        .find(|e| e.storage_path == *file_path)
                    {
                        all_files.insert(file_path.clone(), entry.clone());
                    }
                }
            }
        }

        // 2. Process Premint/Cached Files
        // These are files uploaded but not yet minted into an NFT.
        // They are stored directly in premint_cache keyed by file_path.
        for (path, entry) in &self.premint_cache {
            // Only insert if not already present
            if !all_files.contains_key(path) {
                all_files.insert(path.clone(), entry.clone());
            }
        }

        all_files
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct NftPublicRecord {
    pub entries: HashMap<String, PublicEntry>,
    pub burned_at_ns: Option<u64>,
}

impl Default for NftPublicRecord {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            burned_at_ns: None,
        }
    }
}

impl NftPublicRecord {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct PublicEntry {
    pub state: UploadState,
    pub hash: String,
    pub file_size: u64,
    pub storage_canister_id: Principal,
    pub storage_path: String,
    #[serde(default)]
    pub pending_upload: Option<PendingUpload>,
    pub format_version: u8,
    #[serde(default)]
    pub created_at_ns: u64,
}
