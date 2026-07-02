use crate::append_file;
use crate::append_file::AppendFileError;
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

// NOTE: there are a lot of denormalizing introduced to allow faster access to files needed
#[derive(Serialize, Deserialize, Clone)]
pub struct PublicContentSystem {
    pub nft_public: HashMap<Nat, NftPublicRecord>, // all minted NFTs with public content, indexed by token_id
    pub temp_file_cache: HashMap<String, PublicEntry>, // NOTE: indexing by file_path
    pub file_to_nfts: HashMap<String, HashSet<Nat>>, // all files and NFTs that are using it
    pub nft_to_files: HashMap<Nat, HashSet<String>>, // nft to file pathes mapping
    pub all_files_index: HashMap<String, PublicEntry>, // files that was already saved
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
        self.temp_file_cache.get(file_path).cloned()
    }

    pub fn remove_entry_by_path(&mut self, file_path: &String) -> Result<(), PublicContentError> {
        // Guard: If it's already active/linked to an NFT, it cannot be raw-deleted from here
        if self.file_to_nfts.contains_key(file_path) {
            return Err(PublicContentError::InvalidStateTransition);
        }

        // Remove from the cache map
        match self.temp_file_cache.remove(file_path) {
            Some(_) => Ok(()),
            None => Err(PublicContentError::NotFound),
        }
    }

    pub fn init_upload_validate(
        &self,
        file_path: &String,
        file_size: u64,
    ) -> crate::types::init_upload::Response {
        // Reject if already in the premint cache (upload in progress / pending mint)
        if self.temp_file_cache.contains_key(file_path) {
            return Err(InitUploadError::FileAlreadyExists);
        }

        // Reject if already minted into an active NFT — prevents path collisions
        if self.file_to_nfts.contains_key(file_path) {
            return Err(InitUploadError::FileAlreadyExists);
        }

        if file_size > MAX_CONTENT_SIZE {
            return Err(InitUploadError::ContentTooLarge);
        }

        Ok(InitUploadResp {})
    }

    // MODIFIED: Uses file_path string key instead of hash
    pub fn init_upload_register(
        &mut self,
        file_path: String,
        storage_canister_id: Principal,
        storage_path: String,
        expected_chunks: usize,
        file_size: u64,
        timestamp: u64,
    ) -> crate::types::init_upload::Response {
        if self.temp_file_cache.contains_key(&file_path) {
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
                timestamp_ns: timestamp,
            }),
            format_version: 1,
            created_at_ns: timestamp,
        };

        self.temp_file_cache.insert(file_path, entry);
        Ok(InitUploadResp {})
    }

    pub fn upload_chunk_register(
        &mut self,
        file_path: &String,
        chunk_index: Nat,
        data: Vec<u8>,
    ) -> Result<(), StoreChunkError> {
        let entry = if self.temp_file_cache.contains_key(file_path) {
            self.temp_file_cache
                .get_mut(file_path)
                .ok_or(StoreChunkError::InvalidFilePath)?
        } else {
            let token_ids = self
                .file_to_nfts
                .get(file_path)
                .ok_or(StoreChunkError::InvalidFilePath)?;

            let mut target_token_id = None;
            let mut target_entry_name = None;

            for tid in token_ids {
                if let Some(rec) = self.nft_public.get(tid) {
                    if let Some((name, e)) = rec
                        .entries
                        .iter()
                        .find(|(_, e)| &e.storage_path == file_path)
                    {
                        if e.state == UploadState::ReuploadInit
                            || e.state == UploadState::InProgress
                        {
                            target_token_id = Some(tid.clone());
                            target_entry_name = Some(name.clone());
                            break;
                        }
                    }
                }
            }

            let t_id = target_token_id.ok_or(StoreChunkError::InvalidFilePath)?;
            let e_name = target_entry_name.ok_or(StoreChunkError::InvalidFilePath)?;

            self.nft_public
                .get_mut(&t_id)
                .and_then(|rec| rec.entries.get_mut(&e_name))
                .ok_or(StoreChunkError::InvalidFilePath)?
        };

        if entry.state != UploadState::Init
            && entry.state != UploadState::InProgress
            && entry.state != UploadState::ReuploadInit
        {
            return Err(StoreChunkError::InvalidStateTransition);
        }

        let pending = entry
            .pending_upload
            .as_mut()
            .ok_or(StoreChunkError::InvalidStateTransition)?;

        let expected_chunks_nat = Nat::from(pending.expected_chunks);
        if chunk_index >= expected_chunks_nat {
            return Err(StoreChunkError::InvalidChunkId);
        }

        if data.len() > CHUNK_SIZE {
            return Err(StoreChunkError::InvalidChunkData);
        }

        pending.received_chunks.insert(chunk_index, data);

        if entry.state == UploadState::Init || entry.state == UploadState::ReuploadInit {
            entry.state = UploadState::InProgress;
        }

        Ok(())
    }

    pub fn finalize_upload_register(
        &mut self,
        file_path: &String,
    ) -> Result<(), FinalizeUploadError> {
        let entry = if self.temp_file_cache.contains_key(file_path) {
            self.temp_file_cache
                .get_mut(file_path)
                .ok_or(FinalizeUploadError::UploadNotStarted)?
        } else {
            // FIX: Match the targeted search logic from chunk registration
            let token_ids = self
                .file_to_nfts
                .get(file_path)
                .ok_or(FinalizeUploadError::UploadNotStarted)?;
            let mut target = None;

            for tid in token_ids {
                if let Some(rec) = self.nft_public.get(tid) {
                    if let Some((name, e)) = rec
                        .entries
                        .iter()
                        .find(|(_, e)| &e.storage_path == file_path)
                    {
                        if e.state == UploadState::InProgress || e.state == UploadState::Init {
                            target = Some((tid.clone(), name.clone()));
                            break;
                        }
                    }
                }
            }

            let (t_id, e_name) = target.ok_or(FinalizeUploadError::UploadNotStarted)?;
            self.nft_public
                .get_mut(&t_id)
                .and_then(|rec| rec.entries.get_mut(&e_name))
                .ok_or(FinalizeUploadError::UploadNotStarted)?
        };

        if entry.state != UploadState::Init && entry.state != UploadState::InProgress {
            return Err(FinalizeUploadError::UploadAlreadyFinalized);
        }

        let pending = entry
            .pending_upload
            .as_ref()
            .ok_or(FinalizeUploadError::UploadNotStarted)?;

        if pending.received_chunks.len() != pending.expected_chunks {
            return Err(FinalizeUploadError::IncompleteUpload);
        }

        let actual_size: u64 = pending
            .received_chunks
            .values()
            .map(|c: &Vec<u8>| c.len() as u64)
            .sum();

        if actual_size != entry.file_size {
            return Err(FinalizeUploadError::FileSizeMismatch);
        }

        entry.state = UploadState::Finalized;
        entry.pending_upload = None;

        Ok(())
    }

    pub fn cancel_upload_register(&mut self, file_path: &String) -> crate::cancel_upload::Response {
        self.temp_file_cache.remove(file_path);
        Ok(CancelUploadResp {})
    }

    pub fn append_register(
        &mut self,
        entries_to_append_paths: HashMap<String, String>,
        token_id: Nat,
    ) -> append_file::Response {
        if self.nft_public.contains_key(&token_id) {
            return Err(AppendFileError::FileAlreadyExists);
        }

        let mut public_record = NftPublicRecord::new();

        for (entry_name, file_path) in entries_to_append_paths {
            let mut entry = match self.temp_file_cache.remove(&file_path) {
                Some(cached_entry) => cached_entry,
                None => self
                    .nft_public
                    .values()
                    .find_map(|record| record.entries.values().find(|e| e.hash == file_path))
                    .cloned()
                    .ok_or(AppendFileError::NotFound)?,
            };

            if entry.state != UploadState::Finalized {
                return Err(AppendFileError::InvalidStateTransition);
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
            let mut entry = match self.temp_file_cache.remove(&file_path) {
                Some(cached_entry) => cached_entry,
                None => {
                    let shared_token_id = self
                        .file_to_nfts
                        .get(&file_path)
                        .and_then(|set| set.iter().next())
                        .ok_or(PublicContentError::NotFound)?;

                    self.nft_public
                        .get(shared_token_id)
                        .and_then(|record| {
                            record
                                .entries
                                .values()
                                .find(|e| e.storage_path == file_path)
                                .cloned()
                        })
                        .ok_or(PublicContentError::NotFound)?
                }
            };

            if entry.state != UploadState::Finalized {
                return Err(PublicContentError::InvalidStateTransition);
            }

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

    pub fn burn_public_content(&mut self, token_id: &Nat, timestamp: u64) -> Vec<String> {
        let mut files_to_delete = Vec::new();

        if let Some(record) = self.nft_public.get_mut(token_id) {
            record.burned_at_ns = Some(timestamp);
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
                            .filter(|(_name, entry)| {
                                // Safe to delete only when no live NFT references this path
                                !self.file_to_nfts.contains_key(&entry.storage_path)
                            })
                            .map(|(name, entry)| {
                                (
                                    name.clone(),
                                    entry.storage_canister_id,
                                    entry.storage_path.clone(),
                                )
                            })
                            .collect();

                        if files.is_empty() {
                            None
                        } else {
                            Some((token_id.clone(), files))
                        }
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

        // 1. Process active/minted files. Since files can be shared across multiple NFTs,
        // we iterate over file_to_nfts keys to ensure unique paths.
        let mut combined: Vec<(String, UploadState)> = self
            .file_to_nfts
            .iter()
            .filter_map(|(file_path, nft_set)| {
                let token_id_ref = nft_set.iter().next()?;
                let record = self.nft_public.get(token_id_ref)?;
                let entry = record
                    .entries
                    .values()
                    .find(|e| e.storage_path == *file_path)?;
                Some((file_path.clone(), entry.state.clone()))
            })
            .collect();

        // 2. Process premint/cached files (only those not already in active files)
        for (path, entry) in &self.temp_file_cache {
            if !self.file_to_nfts.contains_key(path) {
                combined.push((path.clone(), entry.state.clone()));
            }
        }

        // Sort by path to ensure deterministic pagination
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
        // They are stored directly in temp_file_cache keyed by file_path.
        for (path, entry) in &self.temp_file_cache {
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

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct PendingUpload {
    pub expected_chunks: usize,
    pub received_chunks: HashMap<Nat, Vec<u8>>,
    pub chunk_size: usize,
    pub timestamp_ns: u64,
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

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    fn dummy_principal() -> Principal {
        Principal::from_text("aaaaa-aa").unwrap()
    }

    fn make_system() -> PublicContentSystem {
        PublicContentSystem {
            nft_public: HashMap::new(),
            temp_file_cache: HashMap::new(),
            file_to_nfts: HashMap::new(),
            nft_to_files: HashMap::new(),
            all_files_index: HashMap::new(),
        }
    }

    fn register_finalized_upload(system: &mut PublicContentSystem, file_path: &str, data: Vec<u8>) {
        let file_size = data.len() as u64;
        system
            .init_upload_register(
                file_path.to_string(),
                dummy_principal(),
                file_path.to_string(),
                1, // 1 expected chunk
                file_size,
                0,
            )
            .unwrap();

        system
            .upload_chunk_register(&file_path.to_string(), Nat::from(0u64), data)
            .unwrap();

        system
            .finalize_upload_register(&file_path.to_string())
            .unwrap();
    }

    // ---------------------------------------------------------------------------
    // init_upload_validate
    // ---------------------------------------------------------------------------

    #[test]
    fn init_upload_validate_accepts_new_file() {
        let system = make_system();
        let result = system.init_upload_validate(&"file/path.png".to_string(), 100);
        assert!(result.is_ok());
    }

    #[test]
    fn init_upload_validate_rejects_file_in_cache() {
        let mut system = make_system();
        let path = "file/path.png".to_string();
        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 1, 100, 0)
            .unwrap();

        let result = system.init_upload_validate(&path, 100);
        assert!(matches!(
            result,
            Err(crate::types::init_upload::InitUploadError::FileAlreadyExists)
        ));
    }

    #[test]
    fn init_upload_validate_rejects_file_already_minted() {
        let mut system = make_system();
        let path = "file/path.png".to_string();
        let token_id = Nat::from(1u64);

        register_finalized_upload(&mut system, &path, vec![0u8; 100]);

        let mut entries = HashMap::new();
        entries.insert("main".to_string(), path.clone());
        system.mint_public_content(entries, token_id).unwrap();

        let result = system.init_upload_validate(&path, 100);
        assert!(matches!(
            result,
            Err(crate::types::init_upload::InitUploadError::FileAlreadyExists)
        ));
    }

    #[test]
    fn init_upload_validate_rejects_oversized_file() {
        let system = make_system();
        let result = system.init_upload_validate(&"big.bin".to_string(), MAX_CONTENT_SIZE + 1);
        assert!(matches!(
            result,
            Err(crate::types::init_upload::InitUploadError::ContentTooLarge)
        ));
    }

    // ---------------------------------------------------------------------------
    // init_upload_register
    // ---------------------------------------------------------------------------

    #[test]
    fn init_upload_register_adds_entry_to_cache() {
        let mut system = make_system();
        let path = "audio/track.mp3".to_string();

        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 3, 300, 42)
            .unwrap();

        let entry = system.temp_file_cache.get(&path).unwrap();
        assert_eq!(entry.state, UploadState::Init);
        assert_eq!(entry.file_size, 300);
        assert_eq!(entry.created_at_ns, 42);
        assert_eq!(entry.pending_upload.as_ref().unwrap().expected_chunks, 3);
    }

    #[test]
    fn init_upload_register_rejects_duplicate_path() {
        let mut system = make_system();
        let path = "audio/track.mp3".to_string();

        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 1, 100, 0)
            .unwrap();

        let result =
            system.init_upload_register(path.clone(), dummy_principal(), path.clone(), 1, 100, 0);
        assert!(matches!(
            result,
            Err(crate::types::init_upload::InitUploadError::FileAlreadyExists)
        ));
    }

    // ---------------------------------------------------------------------------
    // upload_chunk_register
    // ---------------------------------------------------------------------------

    #[test]
    fn upload_chunk_register_transitions_state_to_in_progress() {
        let mut system = make_system();
        let path = "img/photo.jpg".to_string();
        let data = vec![1u8; 100];

        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 2, 200, 0)
            .unwrap();

        system
            .upload_chunk_register(&path, Nat::from(0u64), data)
            .unwrap();

        let entry = system.temp_file_cache.get(&path).unwrap();
        assert_eq!(entry.state, UploadState::InProgress);
    }

    #[test]
    fn upload_chunk_register_stores_multiple_chunks() {
        let mut system = make_system();
        let path = "doc/file.pdf".to_string();

        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 3, 300, 0)
            .unwrap();

        for i in 0u64..3 {
            system
                .upload_chunk_register(&path, Nat::from(i), vec![i as u8; 100])
                .unwrap();
        }

        let chunks = &system
            .temp_file_cache
            .get(&path)
            .unwrap()
            .pending_upload
            .as_ref()
            .unwrap()
            .received_chunks;
        assert_eq!(chunks.len(), 3);
    }

    #[test]
    fn upload_chunk_register_rejects_out_of_bounds_chunk() {
        let mut system = make_system();
        let path = "img/photo.jpg".to_string();

        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 1, 100, 0)
            .unwrap();

        // chunk_index 1 is out of bounds for expected_chunks = 1
        let result = system.upload_chunk_register(&path, Nat::from(1u64), vec![0u8; 100]);
        assert!(matches!(
            result,
            Err(crate::types::store_chunk::StoreChunkError::InvalidChunkId)
        ));
    }

    #[test]
    fn upload_chunk_register_rejects_oversized_chunk() {
        let mut system = make_system();
        let path = "img/photo.jpg".to_string();

        system
            .init_upload_register(
                path.clone(),
                dummy_principal(),
                path.clone(),
                1,
                CHUNK_SIZE as u64,
                0,
            )
            .unwrap();

        let oversized = vec![0u8; CHUNK_SIZE + 1];
        let result = system.upload_chunk_register(&path, Nat::from(0u64), oversized);
        assert!(matches!(
            result,
            Err(crate::types::store_chunk::StoreChunkError::InvalidChunkData)
        ));
    }

    #[test]
    fn upload_chunk_register_rejects_unknown_path() {
        let mut system = make_system();
        let result =
            system.upload_chunk_register(&"nonexistent.bin".to_string(), Nat::from(0u64), vec![]);
        assert!(matches!(
            result,
            Err(crate::types::store_chunk::StoreChunkError::InvalidFilePath)
        ));
    }

    // ---------------------------------------------------------------------------
    // finalize_upload_register
    // ---------------------------------------------------------------------------

    #[test]
    fn finalize_upload_register_succeeds_with_all_chunks() {
        let mut system = make_system();
        let path = "vid/clip.mp4".to_string();
        let data = vec![42u8; 512];

        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 1, 512, 0)
            .unwrap();
        system
            .upload_chunk_register(&path, Nat::from(0u64), data)
            .unwrap();
        system.finalize_upload_register(&path).unwrap();

        let entry = system.temp_file_cache.get(&path).unwrap();
        assert_eq!(entry.state, UploadState::Finalized);
        assert!(entry.pending_upload.is_none());
    }

    #[test]
    fn finalize_upload_register_rejects_missing_chunks() {
        let mut system = make_system();
        let path = "vid/clip.mp4".to_string();

        // 2 expected chunks, but we only upload 1
        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 2, 200, 0)
            .unwrap();
        system
            .upload_chunk_register(&path, Nat::from(0u64), vec![0u8; 100])
            .unwrap();

        let result = system.finalize_upload_register(&path);
        assert!(matches!(
            result,
            Err(crate::finalize_upload::FinalizeUploadError::IncompleteUpload)
        ));
    }

    #[test]
    fn finalize_upload_register_rejects_file_size_mismatch() {
        let mut system = make_system();
        let path = "vid/clip.mp4".to_string();

        // Declare 200 bytes but upload 100
        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 1, 200, 0)
            .unwrap();
        system
            .upload_chunk_register(&path, Nat::from(0u64), vec![0u8; 100])
            .unwrap();

        let result = system.finalize_upload_register(&path);
        assert!(matches!(
            result,
            Err(crate::finalize_upload::FinalizeUploadError::FileSizeMismatch)
        ));
    }

    #[test]
    fn finalize_upload_register_rejects_unknown_path() {
        let mut system = make_system();
        let result = system.finalize_upload_register(&"ghost.bin".to_string());
        assert!(matches!(
            result,
            Err(crate::finalize_upload::FinalizeUploadError::UploadNotStarted)
        ));
    }

    // ---------------------------------------------------------------------------
    // cancel_upload_register
    // ---------------------------------------------------------------------------

    #[test]
    fn cancel_upload_register_removes_entry() {
        let mut system = make_system();
        let path = "img/photo.jpg".to_string();

        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 1, 100, 0)
            .unwrap();

        let _ = system.cancel_upload_register(&path);
        assert!(!system.temp_file_cache.contains_key(&path));
    }

    #[test]
    fn cancel_upload_register_is_idempotent_on_unknown_path() {
        let mut system = make_system();
        // Should succeed (returning Ok) even when the path doesn't exist
        let result = system.cancel_upload_register(&"nonexistent.png".to_string());
        assert!(result.is_ok());
    }

    // ---------------------------------------------------------------------------
    // mint_public_content
    // ---------------------------------------------------------------------------

    #[test]
    fn mint_public_content_moves_entry_to_nft_public() {
        let mut system = make_system();
        let path = "img/token.png".to_string();
        let token_id = Nat::from(1u64);

        register_finalized_upload(&mut system, &path, vec![0xAB; 100]);

        let mut entries = HashMap::new();
        entries.insert("thumbnail".to_string(), path.clone());
        system
            .mint_public_content(entries, token_id.clone())
            .unwrap();

        // Removed from cache
        assert!(!system.temp_file_cache.contains_key(&path));

        // Present in nft_public
        assert!(system.nft_public.contains_key(&token_id));
        assert!(system
            .nft_public
            .get(&token_id)
            .unwrap()
            .entries
            .contains_key("thumbnail"));

        // Indexes updated
        assert!(system.file_to_nfts.get(&path).unwrap().contains(&token_id));
        assert!(system.nft_to_files.get(&token_id).unwrap().contains(&path));
    }

    #[test]
    fn mint_public_content_rejects_duplicate_token_id() {
        let mut system = make_system();
        let token_id = Nat::from(1u64);

        let path1 = "img/a.png".to_string();
        register_finalized_upload(&mut system, &path1, vec![1u8; 50]);
        let mut entries1 = HashMap::new();
        entries1.insert("a".to_string(), path1.clone());
        system
            .mint_public_content(entries1, token_id.clone())
            .unwrap();

        // Attempt to mint the same token_id again
        let path2 = "img/b.png".to_string();
        register_finalized_upload(&mut system, &path2, vec![2u8; 50]);
        let mut entries2 = HashMap::new();
        entries2.insert("b".to_string(), path2.clone());
        let result = system.mint_public_content(entries2, token_id);
        assert!(matches!(result, Err(PublicContentError::FileAlreadyExists)));
    }

    #[test]
    fn mint_public_content_rejects_non_finalized_entry() {
        let mut system = make_system();
        let path = "img/partial.png".to_string();
        let token_id = Nat::from(2u64);

        // Register but do NOT finalize
        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 1, 100, 0)
            .unwrap();

        let mut entries = HashMap::new();
        entries.insert("partial".to_string(), path.clone());
        let result = system.mint_public_content(entries, token_id);
        assert!(matches!(
            result,
            Err(PublicContentError::InvalidStateTransition)
        ));
    }

    #[test]
    fn mint_public_content_allows_shared_file_across_nfts() {
        let mut system = make_system();
        let path = "shared/logo.svg".to_string();

        register_finalized_upload(&mut system, &path, vec![0xCC; 64]);

        // First mint
        let token1 = Nat::from(10u64);
        let mut e1 = HashMap::new();
        e1.insert("logo".to_string(), path.clone());
        system.mint_public_content(e1, token1.clone()).unwrap();

        // Second mint reusing the same file (it's now in nft_public, not cache)
        let token2 = Nat::from(11u64);
        let mut e2 = HashMap::new();
        e2.insert("logo".to_string(), path.clone());
        system.mint_public_content(e2, token2.clone()).unwrap();

        // Both NFTs reference the shared file
        let set = system.file_to_nfts.get(&path).unwrap();
        assert!(set.contains(&token1));
        assert!(set.contains(&token2));
    }

    // ---------------------------------------------------------------------------
    // append_register
    // ---------------------------------------------------------------------------

    #[test]
    fn append_register_links_entry_to_new_token_id() {
        let mut system = make_system();
        let path = "doc/readme.md".to_string();
        let token_id = Nat::from(99u64);

        register_finalized_upload(&mut system, &path, vec![0u8; 50]);

        let mut entries = HashMap::new();
        entries.insert("readme".to_string(), path.clone());
        system.append_register(entries, token_id.clone()).unwrap();

        assert!(system.nft_public.contains_key(&token_id));
        assert!(system.file_to_nfts.get(&path).unwrap().contains(&token_id));
        assert!(system.nft_to_files.get(&token_id).unwrap().contains(&path));
    }

    #[test]
    fn append_register_rejects_duplicate_token() {
        let mut system = make_system();
        let path = "doc/readme.md".to_string();
        let token_id = Nat::from(99u64);

        register_finalized_upload(&mut system, &path, vec![0u8; 50]);
        let mut entries = HashMap::new();
        entries.insert("readme".to_string(), path.clone());
        system.append_register(entries, token_id.clone()).unwrap();

        // Try appending again with the same token — should fail
        let path2 = "doc/other.md".to_string();
        register_finalized_upload(&mut system, &path2, vec![0u8; 30]);
        let mut entries2 = HashMap::new();
        entries2.insert("other".to_string(), path2);
        let result = system.append_register(entries2, token_id);
        assert!(matches!(
            result,
            Err(crate::append_file::AppendFileError::FileAlreadyExists)
        ));
    }

    // ---------------------------------------------------------------------------
    // burn_public_content
    // ---------------------------------------------------------------------------

    #[test]
    fn burn_public_content_marks_burned_at_and_removes_indexes() {
        let mut system = make_system();
        let path = "img/burn_me.png".to_string();
        let token_id = Nat::from(5u64);

        register_finalized_upload(&mut system, &path, vec![0u8; 100]);
        let mut entries = HashMap::new();
        entries.insert("img".to_string(), path.clone());
        system
            .mint_public_content(entries, token_id.clone())
            .unwrap();

        let files_to_delete = system.burn_public_content(&token_id, 12345);

        // The NFT record should be marked with burned_at_ns
        assert_eq!(
            system.nft_public.get(&token_id).unwrap().burned_at_ns,
            Some(12345)
        );

        // nft_to_files index cleared for this token
        assert!(!system.nft_to_files.contains_key(&token_id));

        // File is returned for deletion since no other NFT references it
        assert_eq!(files_to_delete, vec![path.clone()]);

        // file_to_nfts cleared for the orphaned file
        assert!(!system.file_to_nfts.contains_key(&path));
    }

    #[test]
    fn burn_public_content_keeps_shared_file_when_other_nft_references_it() {
        let mut system = make_system();
        let path = "shared/asset.png".to_string();

        register_finalized_upload(&mut system, &path, vec![0xAA; 64]);

        let token1 = Nat::from(1u64);
        let mut e1 = HashMap::new();
        e1.insert("a".to_string(), path.clone());
        system.mint_public_content(e1, token1.clone()).unwrap();

        let token2 = Nat::from(2u64);
        let mut e2 = HashMap::new();
        e2.insert("a".to_string(), path.clone());
        system.mint_public_content(e2, token2.clone()).unwrap();

        // Burn only token1 — file should NOT be scheduled for deletion
        let files_to_delete = system.burn_public_content(&token1, 999);
        assert!(files_to_delete.is_empty());

        // file_to_nfts still holds token2
        let remaining = system.file_to_nfts.get(&path).unwrap();
        assert!(remaining.contains(&token2));
        assert!(!remaining.contains(&token1));
    }

    // ---------------------------------------------------------------------------
    // collect_expired_burned
    // ---------------------------------------------------------------------------

    #[test]
    fn collect_expired_burned_returns_expired_entries() {
        let mut system = make_system();
        let path = "img/old.png".to_string();
        let token_id = Nat::from(7u64);

        register_finalized_upload(&mut system, &path, vec![0u8; 100]);
        let mut entries = HashMap::new();
        entries.insert("old".to_string(), path.clone());
        system
            .mint_public_content(entries, token_id.clone())
            .unwrap();

        // Burn at t=100 with threshold 50 → expired when now >= 150
        system.burn_public_content(&token_id, 100);
        let expired = system.collect_expired_burned(200, 50);

        // No files to collect because burn_public_content already removed file_to_nfts
        // and collect_expired_burned only includes files NOT in file_to_nfts.
        // The file WAS removed from file_to_nfts during burn, so it should appear here.
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].0, token_id);
    }

    #[test]
    fn collect_expired_burned_ignores_not_yet_expired_entries() {
        let mut system = make_system();
        let path = "img/recent.png".to_string();
        let token_id = Nat::from(8u64);

        register_finalized_upload(&mut system, &path, vec![0u8; 100]);
        let mut entries = HashMap::new();
        entries.insert("recent".to_string(), path.clone());
        system
            .mint_public_content(entries, token_id.clone())
            .unwrap();

        // Burned at t=1000, threshold=500 → only expired when now >= 1500
        system.burn_public_content(&token_id, 1000);
        // now = 1499 → should NOT be expired
        let expired = system.collect_expired_burned(1499, 500);
        assert!(expired.is_empty());
    }

    #[test]
    fn collect_expired_burned_skips_shared_files() {
        let mut system = make_system();
        let path = "shared/img.png".to_string();

        register_finalized_upload(&mut system, &path, vec![0xBB; 64]);

        let token1 = Nat::from(20u64);
        let token2 = Nat::from(21u64);

        let mut e1 = HashMap::new();
        e1.insert("img".to_string(), path.clone());
        system.mint_public_content(e1, token1.clone()).unwrap();

        let mut e2 = HashMap::new();
        e2.insert("img".to_string(), path.clone());
        system.mint_public_content(e2, token2.clone()).unwrap();

        // Burn token1 — file_to_nfts still contains token2
        system.burn_public_content(&token1, 10);

        // Expired, but file is still shared → no files should be in result
        let expired = system.collect_expired_burned(100, 5);
        let record = expired.iter().find(|(id, _)| id == &token1);
        if let Some((_, files)) = record {
            assert!(
                files.is_empty(),
                "Shared file must not appear in expired list"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // remove_burned_record
    // ---------------------------------------------------------------------------

    #[test]
    fn remove_burned_record_clears_nft_public_entry() {
        let mut system = make_system();
        let path = "img/gone.png".to_string();
        let token_id = Nat::from(55u64);

        register_finalized_upload(&mut system, &path, vec![0u8; 64]);
        let mut entries = HashMap::new();
        entries.insert("gone".to_string(), path.clone());
        system
            .mint_public_content(entries, token_id.clone())
            .unwrap();

        system.burn_public_content(&token_id, 1);
        system.remove_burned_record(&token_id);

        assert!(!system.nft_public.contains_key(&token_id));
    }

    // ---------------------------------------------------------------------------
    // get_nft_public_entry
    // ---------------------------------------------------------------------------

    #[test]
    fn get_nft_public_entry_returns_correct_entry() {
        let mut system = make_system();
        let path = "img/icon.png".to_string();
        let token_id = Nat::from(1u64);

        register_finalized_upload(&mut system, &path, vec![0xDE; 64]);
        let mut entries = HashMap::new();
        entries.insert("icon".to_string(), path.clone());
        system
            .mint_public_content(entries, token_id.clone())
            .unwrap();

        let entry = system
            .get_nft_public_entry(token_id.clone(), "icon")
            .unwrap();
        assert_eq!(entry.storage_path, path);
    }

    #[test]
    fn get_nft_public_entry_errors_on_missing_token() {
        let system = make_system();
        let result = system.get_nft_public_entry(Nat::from(999u64), "icon");
        assert!(result.is_err());
    }

    #[test]
    fn get_nft_public_entry_errors_on_missing_entry_name() {
        let mut system = make_system();
        let path = "img/icon.png".to_string();
        let token_id = Nat::from(1u64);

        register_finalized_upload(&mut system, &path, vec![0u8; 64]);
        let mut entries = HashMap::new();
        entries.insert("icon".to_string(), path.clone());
        system
            .mint_public_content(entries, token_id.clone())
            .unwrap();

        let result = system.get_nft_public_entry(token_id, "banner");
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------------------
    // get_files_by_nft
    // ---------------------------------------------------------------------------

    #[test]
    fn get_files_by_nft_returns_associated_files() {
        let mut system = make_system();
        let path = "img/photo.png".to_string();
        let token_id = Nat::from(1u64);

        register_finalized_upload(&mut system, &path, vec![0u8; 64]);
        let mut entries = HashMap::new();
        entries.insert("photo".to_string(), path.clone());
        system
            .mint_public_content(entries, token_id.clone())
            .unwrap();

        let files = system.get_files_by_nft(&token_id);
        assert_eq!(files, vec![path]);
    }

    #[test]
    fn get_files_by_nft_returns_empty_for_unknown_token() {
        let system = make_system();
        let files = system.get_files_by_nft(&Nat::from(404u64));
        assert!(files.is_empty());
    }

    // ---------------------------------------------------------------------------
    // get_public_file_by_path
    // ---------------------------------------------------------------------------

    #[test]
    fn get_public_file_by_path_returns_cached_entry() {
        let mut system = make_system();
        let path = "img/cached.png".to_string();

        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 1, 100, 0)
            .unwrap();

        let entry = system.get_public_file_by_path(&path);
        assert!(entry.is_some());
    }

    #[test]
    fn get_public_file_by_path_returns_minted_entry() {
        let mut system = make_system();
        let path = "img/minted.png".to_string();
        let token_id = Nat::from(1u64);

        register_finalized_upload(&mut system, &path, vec![0u8; 64]);
        let mut entries = HashMap::new();
        entries.insert("minted".to_string(), path.clone());
        system
            .mint_public_content(entries, token_id.clone())
            .unwrap();

        let entry = system.get_public_file_by_path(&path);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().storage_path, path);
    }

    #[test]
    fn get_public_file_by_path_returns_none_for_unknown_path() {
        let system = make_system();
        assert!(system.get_public_file_by_path("unknown/file.png").is_none());
    }

    // ---------------------------------------------------------------------------
    // remove_entry_by_path
    // ---------------------------------------------------------------------------

    #[test]
    fn remove_entry_by_path_removes_cached_entry() {
        let mut system = make_system();
        let path = "temp/upload.bin".to_string();

        system
            .init_upload_register(path.clone(), dummy_principal(), path.clone(), 1, 100, 0)
            .unwrap();

        system.remove_entry_by_path(&path).unwrap();
        assert!(!system.temp_file_cache.contains_key(&path));
    }

    #[test]
    fn remove_entry_by_path_rejects_minted_file() {
        let mut system = make_system();
        let path = "img/active.png".to_string();
        let token_id = Nat::from(1u64);

        register_finalized_upload(&mut system, &path, vec![0u8; 64]);
        let mut entries = HashMap::new();
        entries.insert("active".to_string(), path.clone());
        system
            .mint_public_content(entries, token_id.clone())
            .unwrap();

        let result = system.remove_entry_by_path(&path);
        assert!(matches!(
            result,
            Err(PublicContentError::InvalidStateTransition)
        ));
    }

    #[test]
    fn remove_entry_by_path_returns_not_found_for_unknown_path() {
        let mut system = make_system();
        let result = system.remove_entry_by_path(&"ghost.png".to_string());
        assert!(matches!(result, Err(PublicContentError::NotFound)));
    }

    // ---------------------------------------------------------------------------
    // get_paginated_uploads
    // ---------------------------------------------------------------------------

    #[test]
    fn get_paginated_uploads_includes_cached_and_minted_entries() {
        let mut system = make_system();

        // Cached (not minted)
        let path_cached = "img/cached.png".to_string();
        system
            .init_upload_register(
                path_cached.clone(),
                dummy_principal(),
                path_cached.clone(),
                1,
                100,
                0,
            )
            .unwrap();

        // Minted
        let path_minted = "img/minted.png".to_string();
        let token_id = Nat::from(1u64);
        register_finalized_upload(&mut system, &path_minted, vec![0u8; 64]);
        let mut entries = HashMap::new();
        entries.insert("minted".to_string(), path_minted.clone());
        system.mint_public_content(entries, token_id).unwrap();

        let page = system.get_paginated_uploads(None, None);
        assert!(page.contains_key(&path_cached));
        assert!(page.contains_key(&path_minted));
    }

    #[test]
    fn get_paginated_uploads_respects_skip_and_take() {
        let mut system = make_system();

        // Insert 5 cached entries
        for i in 0..5u64 {
            let path = format!("img/{i:04}.png");
            system
                .init_upload_register(path.clone(), dummy_principal(), path.clone(), 1, 10, 0)
                .unwrap();
        }

        // Skip 2, take 2
        let page = system.get_paginated_uploads(Some(Nat::from(2u64)), Some(Nat::from(2u64)));
        assert_eq!(page.len(), 2);
    }

    // ---------------------------------------------------------------------------
    // get_all_files
    // ---------------------------------------------------------------------------

    #[test]
    fn get_all_files_returns_both_cached_and_minted() {
        let mut system = make_system();

        let path_cached = "img/cached.png".to_string();
        system
            .init_upload_register(
                path_cached.clone(),
                dummy_principal(),
                path_cached.clone(),
                1,
                100,
                0,
            )
            .unwrap();

        let path_minted = "img/minted.png".to_string();
        let token_id = Nat::from(1u64);
        register_finalized_upload(&mut system, &path_minted, vec![0u8; 64]);
        let mut entries = HashMap::new();
        entries.insert("minted".to_string(), path_minted.clone());
        system.mint_public_content(entries, token_id).unwrap();

        let all = system.get_all_files();
        assert!(all.contains_key(&path_cached));
        assert!(all.contains_key(&path_minted));
    }

    #[test]
    fn get_all_files_deduplicates_shared_files() {
        let mut system = make_system();
        let path = "shared/logo.svg".to_string();

        register_finalized_upload(&mut system, &path, vec![0xCC; 64]);

        for i in 1u64..=3 {
            let token_id = Nat::from(i);
            let mut e = HashMap::new();
            e.insert("logo".to_string(), path.clone());
            system.mint_public_content(e, token_id).unwrap();
        }

        let all = system.get_all_files();
        // Shared file appears only once
        assert_eq!(all.iter().filter(|(k, _)| *k == &path).count(), 1);
    }
}
