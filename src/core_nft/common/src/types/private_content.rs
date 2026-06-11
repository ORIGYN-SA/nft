use crate::AccessRights;
use candid::Nat;
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MAX_PRIVATE_CONTENT_SIZE: u64 = 100 * 1024 * 1024; // 100MB
pub const MAX_READERS_PER_ENTRY: usize = 100;
pub const CHUNK_SIZE: usize = 1024 * 1024; // 1MB

pub type Sha256Hash = [u8; 32];

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReaderInfo {
    pub rights: AccessRights,
    pub alias: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PrivateContentStatus {
    PendingUpload,
    PendingMinting,
    Active,
    PendingReencryption,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum EncryptionMode {
    AES256,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum PrivateContentError {
    AlreadyExists,
    InvalidStateTransition,
    NotEnabled,
    NotFound,
    Unauthorized,
    InvalidStatus,
    TooManyReaders,
    ContentTooLarge,
    InvalidChunk,
    StorageError(String),
    VetkdNotConfigured,
    VetkdError(String),
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct PendingUpload {
    pub expected_chunks: usize,
    pub received_chunks: HashMap<usize, Vec<u8>>,
    pub chunk_size: usize,
    pub timestamp_ns: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct ReaderDetail {
    pub principal: Principal,
    pub rights: AccessRights,
    pub alias: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct EntryDetailResp {
    pub name: String,
    pub status: PrivateContentStatus,
    pub readers: Option<Vec<ReaderDetail>>,
    pub plaintext_hash: Sha256Hash,
    pub plaintext_size: u64,
    pub storage_canister_id: Principal,
    pub storage_path: String,
}

// Helper functions
pub fn construct_canonical_identity(readers: &HashMap<Principal, ReaderInfo>) -> Vec<u8> {
    let mut principals: Vec<_> = readers.keys().collect();
    principals.sort();
    let joined = principals
        .iter()
        .map(|p| p.to_text())
        .collect::<Vec<_>>()
        .join(",");
    joined.into_bytes()
}

pub fn resolve_readers(
    entry_readers: &HashMap<Principal, ReaderInfo>,
    default_readers: &HashMap<Principal, ReaderInfo>,
) -> HashMap<Principal, ReaderInfo> {
    if entry_readers.is_empty() {
        default_readers.clone()
    } else {
        entry_readers.clone()
    }
}

pub fn readers_equal(
    a: &HashMap<Principal, ReaderInfo>,
    b: &HashMap<Principal, ReaderInfo>,
) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().all(|(k, v)| {
        b.get(k)
            .map_or(false, |bv| v.rights == bv.rights && v.alias == bv.alias)
    })
}

use ic_cdk::management_canister::VetKDCurve;
use ic_cdk::management_canister::VetKDDeriveKeyArgs;
use ic_cdk::management_canister::VetKDKeyId;
use ic_cdk::management_canister::VetKDPublicKeyArgs;

#[derive(Serialize, Deserialize, Clone)]
pub struct PrivateContentSystem {
    pub nft_private: HashMap<Nat, NftPrivateRecord>,
    pub premint_cache: HashMap<Sha256Hash, NftPrivateRecord>, // NOTE: indexing happens by plaintext_hash. Can be changed to file_hash
    pub config: PrivateContentConfig,
}

impl PrivateContentSystem {
    pub fn get_nft_private_entry(
        &self,
        token_id: Nat,
        entry_name: &str,
    ) -> Result<PrivateEntry, String> {
        self.nft_private
            .get(&token_id)
            .ok_or_else(|| format!("NFT {token_id} not found"))?
            .entries
            .get(entry_name)
            .cloned()
            .ok_or_else(|| format!("Private entry '{entry_name}' not found for NFT {token_id}"))
    }

    // NOTE: full file size (should stay the same for reencrypted content), not just chunk size
    // Validate parameters for private content upload initialization.
    // Checks content size, AES block alignment, and cache conflicts.
    pub fn init_premint_validate(
        &self,
        hash: &Sha256Hash,
        encryption_mode: EncryptionMode,
        file_size: u64,
    ) -> Result<(), PrivateContentError> {
        if self.premint_cache.contains_key(hash) {
            return Err(PrivateContentError::AlreadyExists);
        }

        if file_size > MAX_PRIVATE_CONTENT_SIZE {
            return Err(PrivateContentError::ContentTooLarge);
        }

        match encryption_mode {
            EncryptionMode::AES256 => {
                if file_size % 16 != 0 {
                    return Err(PrivateContentError::StorageError(
                        "Encrypted content size must be divisible by 16 bytes (AES block size)"
                            .to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    // Store a validated private content entry in the premint cache.
    pub fn init_premint_store(
        &mut self,
        hash: Sha256Hash,
        salt: Vec<u8>,
        default_readers: HashMap<Principal, ReaderInfo>,
        entry_name: String,
        storage_canister_id: Principal,
        storage_path: String,
        encryption_mode: EncryptionMode,
        plaintext_size: u64,
        expected_chunks: usize,
        file_size: u64,
    ) -> Result<(), PrivateContentError> {
        // Re-check existence as a safety measure
        if self.premint_cache.contains_key(&hash) {
            return Err(PrivateContentError::AlreadyExists);
        }

        // FIXME: not only default readers
        let canonical_identity = construct_canonical_identity(&default_readers);

        let entry = PrivateEntry {
            status: PrivateContentStatus::PendingUpload,
            readers: default_readers.clone(),
            hash,
            salt,
            plaintext_size,
            file_size,
            encryption_mode,
            canonical_identity,
            previous_canonical_identity: None,
            storage_canister_id,
            storage_path,
            pending_upload: Some(PendingUpload {
                expected_chunks,
                received_chunks: HashMap::new(),
                chunk_size: CHUNK_SIZE,
                timestamp_ns: ic_cdk::api::time(),
            }),
            format_version: 1,
        };

        let mut record = NftPrivateRecord::new();
        record.default_readers = default_readers;
        record.entries.insert(entry_name, entry);

        self.premint_cache.insert(hash, record);
        Ok(())
    }

    // Helper to upload a chunk to a pending entry in premint_cache
    pub fn upload_chunk(
        &mut self,
        plaintext_hash: &Sha256Hash,
        entry_name: &str,
        chunk_index: usize,
        data: Vec<u8>,
    ) -> Result<(), PrivateContentError> {
        let record = self
            .premint_cache
            .get_mut(plaintext_hash)
            .ok_or(PrivateContentError::NotFound)?;
        let entry = record
            .entries
            .get_mut(entry_name)
            .ok_or(PrivateContentError::NotFound)?;

        if entry.status != PrivateContentStatus::PendingUpload {
            return Err(PrivateContentError::InvalidStateTransition);
        }

        let pending = entry
            .pending_upload
            .as_mut()
            .ok_or(PrivateContentError::InvalidStateTransition)?;

        if chunk_index >= pending.expected_chunks {
            return Err(PrivateContentError::InvalidChunk);
        }

        if data.len() > CHUNK_SIZE {
            return Err(PrivateContentError::ContentTooLarge);
        }

        pending.received_chunks.insert(chunk_index, data);

        Ok(())
    }

    // Finalize the upload process. Transitions status from PendingUpload to PendingMinting.
    pub fn finalize_upload(
        &mut self,
        hash: &Sha256Hash,
        entry_name: &str,
    ) -> Result<(), PrivateContentError> {
        let record = self
            .premint_cache
            .get_mut(hash)
            .ok_or(PrivateContentError::NotFound)?;
        let entry = record
            .entries
            .get_mut(entry_name)
            .ok_or(PrivateContentError::NotFound)?;

        if entry.status != PrivateContentStatus::PendingUpload {
            return Err(PrivateContentError::InvalidStateTransition);
        }

        let pending = entry
            .pending_upload
            .as_ref()
            .ok_or(PrivateContentError::InvalidStateTransition)?;

        if pending.received_chunks.len() != pending.expected_chunks {
            return Err(PrivateContentError::InvalidChunk);
        }

        // Verify total size matches expectation
        let actual_size: u64 = pending
            .received_chunks
            .values()
            .map(|c| c.len() as u64)
            .sum();

        if actual_size != entry.file_size {
            return Err(PrivateContentError::ContentTooLarge); // Or specific SizeMismatch error
        }

        // Transition state
        entry.status = PrivateContentStatus::PendingMinting;
        entry.pending_upload = None; // Clear upload buffer to save memory

        Ok(())
    }

    // Cancel an ongoing upload. Removes the entry from premint_cache.
    pub fn cancel_upload(&mut self, entry_name: &str) -> Result<(), PrivateContentError> {
        self.premint_cache
            .retain(|_, record| !record.entries.contains_key(entry_name));
        Ok(())
    }

    // Mint the private content. Moves the record from premint_cache to nft_private.
    pub fn mint_private_content(
        &mut self,
        hash: &Sha256Hash,
        token_id: Nat,
    ) -> Result<(), PrivateContentError> {
        if self.nft_private.contains_key(&token_id) {
            return Err(PrivateContentError::AlreadyExists);
        }

        let record = self
            .premint_cache
            .remove(hash)
            .ok_or(PrivateContentError::NotFound)?;

        // Verify all entries are in PendingMinting or Active state (if re-encryption happened)
        for (_, entry) in record.entries.iter() {
            if entry.status != PrivateContentStatus::PendingMinting
                && entry.status != PrivateContentStatus::Active
            {
                // Allow Active if it was re-encrypted during upload phase, though typically it should be PendingMinting
                // For strict flow, enforce PendingMinting:
                if entry.status != PrivateContentStatus::PendingMinting {
                    return Err(PrivateContentError::InvalidStateTransition);
                }
            }
        }

        self.nft_private.insert(token_id, record);
        Ok(())
    }

    // No duplicates and no nft_owner can be added into readers
    pub fn set_readers(
        &mut self,
        token_id: Nat,
        entry_name: &str,
        nft_owner: Principal,
        readers: HashMap<Principal, ReaderInfo>,
    ) -> Result<(), String> {
        let record = self
            .nft_private
            .get_mut(&token_id)
            .ok_or("No private NFT data was found")?;

        let entry = record
            .entries
            .get_mut(entry_name)
            .ok_or("No private entry was found for given NFT")?;

        entry.set_readers(nft_owner, readers)?;

        Ok(())
    }

    pub fn reencryption_validate(
        &self,
        token_id: &Nat,
        entry_name: &str,
        plaintext_hash: &Sha256Hash,
        plaintext_size: u64,
        file_size: u64,
    ) -> Result<(), PrivateContentError> {
        let record = self
            .nft_private
            .get(token_id)
            .ok_or(PrivateContentError::NotFound)?;

        let entry = record
            .entries
            .get(entry_name)
            .ok_or(PrivateContentError::NotFound)?;

        if entry.status != PrivateContentStatus::PendingReencryption {
            return Err(PrivateContentError::InvalidStateTransition);
        }

        if entry.hash != *plaintext_hash {
            return Err(PrivateContentError::StorageError(
                "Plaintext hash mismatch".to_string(),
            ));
        }

        if entry.plaintext_size != plaintext_size {
            return Err(PrivateContentError::StorageError(
                "Plaintext size mismatch".to_string(),
            ));
        }

        match entry.encryption_mode {
            EncryptionMode::AES256 => {
                if entry.file_size != file_size {
                    return Err(PrivateContentError::StorageError(
                        "Encrypted file size mismatch".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    // Store a validated private content entry in the premint cache.
    pub fn init_reencryption_store(
        &mut self,
        token_id: &Nat,
        entry_name: &str,
        expected_chunks: usize,
    ) -> Result<(), PrivateContentError> {
        let record = self
            .nft_private
            .get_mut(token_id)
            .ok_or(PrivateContentError::NotFound)?;

        let entry = record
            .entries
            .get_mut(entry_name)
            .ok_or(PrivateContentError::NotFound)?;

        entry.pending_upload = Some(PendingUpload {
            expected_chunks,
            received_chunks: HashMap::new(),
            chunk_size: CHUNK_SIZE,
            timestamp_ns: ic_cdk::api::time(),
        });

        Ok(())
    }

    // Helper to upload a chunk to a pending entry in premint_cache
    pub fn reupload_chunk(
        &mut self,
        token_id: &Nat,
        entry_name: &str,
        chunk_index: usize,
        data: Vec<u8>,
    ) -> Result<(), PrivateContentError> {
        let record = self
            .nft_private
            .get_mut(token_id)
            .ok_or(PrivateContentError::NotFound)?;
        let entry = record
            .entries
            .get_mut(entry_name)
            .ok_or(PrivateContentError::NotFound)?;

        if entry.status != PrivateContentStatus::PendingReencryption {
            return Err(PrivateContentError::InvalidStateTransition);
        }

        let pending = entry
            .pending_upload
            .as_mut()
            .ok_or(PrivateContentError::InvalidStateTransition)?;

        if chunk_index >= pending.expected_chunks {
            return Err(PrivateContentError::InvalidChunk);
        }

        if data.len() > CHUNK_SIZE {
            return Err(PrivateContentError::ContentTooLarge);
        }

        pending.received_chunks.insert(chunk_index, data);

        Ok(())
    }

    // Finalize the reupload process. Transitions status from PendingReencryption to PendingMinting.
    pub fn finalize_reupload(
        &mut self,
        hash: &Sha256Hash,
        entry_name: &str,
    ) -> Result<(), PrivateContentError> {
        let record = self
            .premint_cache
            .get_mut(hash)
            .ok_or(PrivateContentError::NotFound)?;
        let entry = record
            .entries
            .get_mut(entry_name)
            .ok_or(PrivateContentError::NotFound)?;

        if entry.status != PrivateContentStatus::PendingReencryption {
            return Err(PrivateContentError::InvalidStateTransition);
        }

        let pending = entry
            .pending_upload
            .as_ref()
            .ok_or(PrivateContentError::InvalidStateTransition)?;

        if pending.received_chunks.len() != pending.expected_chunks {
            return Err(PrivateContentError::InvalidChunk);
        }

        // Verify total size matches expectation
        let actual_size: u64 = pending
            .received_chunks
            .values()
            .map(|c| c.len() as u64)
            .sum();

        if actual_size != entry.file_size {
            return Err(PrivateContentError::ContentTooLarge); // Or specific SizeMismatch error
        }

        // Transition state
        entry.status = PrivateContentStatus::PendingMinting;
        entry.pending_upload = None; // Clear upload buffer to save memory

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrivateContentConfig {
    pub vetkd_key_name: String,
    pub vetkd_context: String,
}

impl PrivateContentConfig {
    fn vetkd_key_id(&self) -> VetKDKeyId {
        VetKDKeyId {
            curve: VetKDCurve::Bls12_381_G2,
            name: self.vetkd_key_name.clone(),
        }
    }

    /// Derives the master public key for IBE encryption.
    pub async fn derive_public_key(&self) -> Result<Vec<u8>, String> {
        let request = VetKDPublicKeyArgs {
            canister_id: None,
            context: self.vetkd_context.clone().into(),
            key_id: self.vetkd_key_id(),
        };

        let reply = ic_cdk::management_canister::vetkd_public_key(&request)
            .await
            .map_err(|e| format!("failed to derive public key: {:?}", e))?;

        Ok(reply.public_key)
    }

    /// Derives a vetkey for a caller-provided identity `input`, encrypted with
    /// `transport_public_key`.
    pub async fn derive_vetkey(
        &self,
        input: Vec<u8>,
        transport_public_key: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        let request = VetKDDeriveKeyArgs {
            input,
            context: self.vetkd_context.clone().into(),
            transport_public_key,
            key_id: self.vetkd_key_id(),
        };

        let reply = ic_cdk::management_canister::vetkd_derive_key(&request)
            .await
            .map_err(|e| format!("failed to derive vetkey: {:?}", e))?;

        Ok(reply.encrypted_key)
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct NftPrivateRecord {
    pub default_readers: HashMap<Principal, ReaderInfo>,
    pub entries: HashMap<String, PrivateEntry>, // TODO: add Merkle tree with all the entries
}

impl NftPrivateRecord {
    pub fn new() -> Self {
        Self {
            default_readers: HashMap::new(),
            entries: HashMap::new(),
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct PrivateEntry {
    pub status: PrivateContentStatus,
    pub readers: HashMap<Principal, ReaderInfo>,
    pub hash: Sha256Hash,
    pub salt: Vec<u8>,
    pub plaintext_size: u64,
    pub file_size: u64,
    pub encryption_mode: EncryptionMode,
    pub canonical_identity: Vec<u8>,
    pub previous_canonical_identity: Option<Vec<u8>>,
    pub storage_canister_id: Principal,
    pub storage_path: String,
    #[serde(default)]
    pub pending_upload: Option<PendingUpload>,
    pub format_version: u8,
}

impl PrivateEntry {
    // No duplicates and no nft_owner can be added into readers
    pub fn set_readers(
        &mut self,
        nft_owner: Principal,
        readers: HashMap<Principal, ReaderInfo>,
    ) -> Result<(), String> {
        let new_readers: HashMap<_, _> = readers
            .into_iter()
            .filter(|(principal, _)| *principal != nft_owner)
            .collect();

        if new_readers.len() > MAX_READERS_PER_ENTRY {
            return Err(format!(
                "Maximum number of readers exceeded: {}",
                MAX_READERS_PER_ENTRY
            ));
        }

        if self.readers != new_readers {
            self.previous_canonical_identity = Some(self.canonical_identity.clone());
            self.readers = new_readers;
            self.canonical_identity = construct_canonical_identity(&self.readers);

            if self.status != PrivateContentStatus::PendingUpload {
                self.status = PrivateContentStatus::PendingReencryption;
            }
        }

        Ok(())
    }
}
