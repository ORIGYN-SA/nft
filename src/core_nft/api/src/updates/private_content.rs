pub use bity_ic_storage_canister_api::updates::cancel_upload as cancel_private_upload;
pub use bity_ic_storage_canister_api::updates::finalize_upload as finalize_private_upload;
pub use bity_ic_storage_canister_api::updates::init_upload as init_private_upload;
pub use bity_ic_storage_canister_api::updates::store_chunk as store_private_chunk;

pub mod derive_vetkey_public_key {
    use candid::CandidType;
    use serde::{Deserialize, Serialize};
    use serde_bytes::ByteBuf;

    pub type Args = ();

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct DeriveVetkeyPublicKeyResp {
        pub public_key: ByteBuf,
    }

    pub type Response = Result<DeriveVetkeyPublicKeyResp, String>;
}

pub mod derive_vetkey {
    use candid::CandidType;
    use serde::{Deserialize, Serialize};
    use serde_bytes::ByteBuf;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub input: ByteBuf,
        pub transport_public_key: ByteBuf,
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct DeriveVetkeyResp {
        pub encrypted_key: ByteBuf,
    }

    pub type Response = Result<DeriveVetkeyResp, String>;
}

pub mod derive_vetkey_by_entry {
    use candid::CandidType;
    use candid::Nat;
    use core_nft_common::EntryName;
    use serde::{Deserialize, Serialize};
    use serde_bytes::ByteBuf;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub token_id: Nat,
        pub entry_name: EntryName,
        pub transport_public_key: ByteBuf,
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct DeriveVetkeyResp {
        pub encrypted_key: ByteBuf,
    }

    pub type Response = Result<DeriveVetkeyResp, String>;
}

pub mod init_private_content_upload {
    use candid::CandidType;
    use candid::Nat;
    use candid::Principal;
    use core_nft_common::types::private_content::EncryptionMode;
    use core_nft_common::EntryName;
    use core_nft_common::ReaderInfo;
    use core_nft_common::Sha256Hash;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        // is used for reuploading reencrypted content
        pub token_id_opt: Option<Nat>,
        pub entry_name: Option<EntryName>,
        // should always stay the same if reencrypted
        pub plaintext_hash: Sha256Hash,
        pub plaintext_size: u64,
        // used to generate a plaintext_hash to obscure the plaintext file
        pub salt: Vec<u8>,
        pub file_hash: Sha256Hash,
        pub readers: HashMap<Principal, ReaderInfo>,
        pub default_readers: HashMap<Principal, ReaderInfo>,
        pub storage_canister_id: Principal,
        pub storage_path: String,
        pub expected_chunks: usize,
        pub chunk_size: Option<u64>,
        pub file_size: u64,
        pub encryption_mode: EncryptionMode,
    }

    use core_nft_common::PrivateContentError;
    pub type Response = Result<(), PrivateContentError>;
}

pub mod store_private_content_chunk {
    use candid::CandidType;
    use candid::Nat;
    use core_nft_common::EntryName;
    use core_nft_common::Sha256Hash;
    use serde::{Deserialize, Serialize};
    use serde_bytes::ByteBuf;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub token_id_opt: Option<Nat>,
        pub entry_name: Option<EntryName>,
        pub plaintext_hash: Sha256Hash,
        pub storage_path: String,
        pub chunk_index: Nat,
        pub chunk_data: ByteBuf,
    }

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum StorePrivateContentChunkError {
        ConcurrentManagementCall,
        NotFound,
        InvalidChunk,
        ContentTooLarge,
        StorageCanisterError(String),
    }

    pub type Response = Result<(), StorePrivateContentChunkError>;
}

use core_nft_common::types::store_chunk;
impl From<store_chunk::StoreChunkError>
    for store_private_content_chunk::StorePrivateContentChunkError
{
    fn from(error: store_chunk::StoreChunkError) -> Self {
        match error {
            store_chunk::StoreChunkError::ConcurrentManagementCall => {
                store_private_content_chunk::StorePrivateContentChunkError::ConcurrentManagementCall
            }
            store_chunk::StoreChunkError::UploadNotInitialized => {
                store_private_content_chunk::StorePrivateContentChunkError::StorageCanisterError(
                    "UploadNotInitialized".to_string(),
                )
            }
            store_chunk::StoreChunkError::UploadAlreadyFinalized => {
                store_private_content_chunk::StorePrivateContentChunkError::StorageCanisterError(
                    "UploadAlreadyFinalized".to_string(),
                )
            }
            store_chunk::StoreChunkError::InvalidStateTransition => {
                store_private_content_chunk::StorePrivateContentChunkError::StorageCanisterError(
                    "InvalidStateTransition".to_string(),
                )
            }
            store_chunk::StoreChunkError::InvalidChunkId => {
                store_private_content_chunk::StorePrivateContentChunkError::InvalidChunk
            }
            store_chunk::StoreChunkError::InvalidFilePath => {
                store_private_content_chunk::StorePrivateContentChunkError::NotFound
            }
            store_chunk::StoreChunkError::InvalidChunkData => {
                store_private_content_chunk::StorePrivateContentChunkError::InvalidChunk
            }
            store_chunk::StoreChunkError::StorageCanisterError(msg) => {
                store_private_content_chunk::StorePrivateContentChunkError::StorageCanisterError(
                    msg,
                )
            }
        }
    }
}

pub mod finalize_private_content_upload {
    use candid::CandidType;
    use candid::Nat;
    use core_nft_common::EntryName;
    use core_nft_common::Sha256Hash;
    use serde::{Deserialize, Serialize};

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub token_id_opt: Option<Nat>,
        pub entry_name: Option<EntryName>,
        pub hash: Sha256Hash,
        pub storage_path: String,
    }

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum FinalizePrivateContentUploadError {
        ConcurrentManagementCall,
        NotFound,
        InvalidStateTransition,
        IncompleteUpload,
        StorageCanisterError(String),
    }

    pub type Response = Result<(), FinalizePrivateContentUploadError>;
}

use core_nft_common::types::finalize_upload;
impl From<finalize_upload::FinalizeUploadError>
    for finalize_private_content_upload::FinalizePrivateContentUploadError
{
    fn from(error: finalize_upload::FinalizeUploadError) -> Self {
        match error {
            finalize_upload::FinalizeUploadError::ConcurrentManagementCall => {
                finalize_private_content_upload::FinalizePrivateContentUploadError::ConcurrentManagementCall
            }
            finalize_upload::FinalizeUploadError::UploadNotStarted => {
                finalize_private_content_upload::FinalizePrivateContentUploadError::StorageCanisterError(
                    "UploadNotStarted".to_string(),
                )
            }
            finalize_upload::FinalizeUploadError::UploadAlreadyFinalized => {
                finalize_private_content_upload::FinalizePrivateContentUploadError::StorageCanisterError(
                    "UploadAlreadyFinalized".to_string(),
                )
            }
            finalize_upload::FinalizeUploadError::IncompleteUpload => {
                finalize_private_content_upload::FinalizePrivateContentUploadError::StorageCanisterError(
                    "IncompleteUpload".to_string(),
                )
            }
            finalize_upload::FinalizeUploadError::StorageCanisterError(msg) => {
                finalize_private_content_upload::FinalizePrivateContentUploadError::StorageCanisterError(
                    msg,
                )
            }
        }
    }
}

pub mod cancel_private_content_upload {

    use candid::CandidType;
    use core_nft_common::Sha256Hash;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
    pub struct Args {
        pub storage_path: String,
        pub entry_hash: Sha256Hash,
    }

    pub type Response = Result<(), CancelPrivateContentUploadError>;

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum CancelPrivateContentUploadError {
        ConcurrentManagementCall,
        UploadNotInitialized,
        InvalidFilePath,
        UploadAlreadyFinalized,
        StorageCanisterError(String),
    }
}

use core_nft_common::types::cancel_upload;
impl From<cancel_upload::CancelUploadError>
    for cancel_private_content_upload::CancelPrivateContentUploadError
{
    fn from(error: cancel_upload::CancelUploadError) -> Self {
        match error {
            cancel_upload::CancelUploadError::ConcurrentManagementCall => {
                cancel_private_content_upload::CancelPrivateContentUploadError::ConcurrentManagementCall
            }
            cancel_upload::CancelUploadError::UploadNotInitialized => {
                cancel_private_content_upload::CancelPrivateContentUploadError::UploadNotInitialized
            }
            cancel_upload::CancelUploadError::UploadAlreadyFinalized => {
                cancel_private_content_upload::CancelPrivateContentUploadError::UploadAlreadyFinalized
            }
            cancel_upload::CancelUploadError::InvalidFilePath => {
                cancel_private_content_upload::CancelPrivateContentUploadError::InvalidFilePath
            }
            cancel_upload::CancelUploadError::StorageCanisterError(msg) => {
                cancel_private_content_upload::CancelPrivateContentUploadError::StorageCanisterError(msg)
            }
        }
    }
}

// 1. From init_private_content_upload::Args -> init_upload::Args
use core_nft_common::types::init_upload;
impl From<init_private_content_upload::Args> for init_upload::Args {
    fn from(args: init_private_content_upload::Args) -> Self {
        init_upload::Args {
            file_path: args.storage_path,
            file_hash: hex::encode(args.file_hash),
            file_size: args.file_size,
            chunk_size: args.chunk_size,
        }
    }
}

// 2. From store_private_content_chunk::Args -> store_chunk::Args
impl From<store_private_content_chunk::Args> for store_chunk::Args {
    fn from(args: store_private_content_chunk::Args) -> Self {
        store_chunk::Args {
            chunk_id: args.chunk_index,
            file_path: args.storage_path,
            chunk_data: args.chunk_data.to_vec(),
        }
    }
}

// 3. From finalize_private_content_upload::Args -> finalize_upload::Args
impl From<finalize_private_content_upload::Args> for finalize_upload::Args {
    fn from(args: finalize_private_content_upload::Args) -> Self {
        finalize_upload::Args {
            file_path: args.storage_path,
        }
    }
}

// 4. From cancel_private_content_upload::Args -> cancel_upload::Args
impl From<cancel_private_content_upload::Args> for cancel_upload::Args {
    fn from(args: cancel_private_content_upload::Args) -> Self {
        cancel_upload::Args {
            file_path: args.storage_path,
        }
    }
}

pub mod set_readers {

    use candid::CandidType;
    use candid::Nat;
    use candid::Principal;
    use core_nft_common::EntryName;
    use core_nft_common::ReaderInfo;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
    pub struct Args {
        pub token_id: Nat,
        pub entry_name: EntryName,
        pub readers: HashMap<Principal, ReaderInfo>,
    }

    pub type Response = Result<(), String>;
}

pub mod __get_private_entry_test {
    use candid::{CandidType, Nat};
    use core_nft_common::EntryName;
    use core_nft_common::PrivateEntry;
    use serde::{Deserialize, Serialize};

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub token_id: Nat,
        pub entry_name: EntryName,
    }

    pub type Response = Result<PrivateEntry, String>;
}

pub mod __get_premint_entry_test {
    use candid::CandidType;
    use core_nft_common::{PrivateEntry, Sha256Hash};
    use serde::{Deserialize, Serialize};

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub hash: Sha256Hash,
    }

    pub type Response = Result<PrivateEntry, String>;
}
