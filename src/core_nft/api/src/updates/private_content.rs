pub use bity_ic_storage_canister_api::updates::cancel_upload as cancel_private_upload;
pub use bity_ic_storage_canister_api::updates::finalize_upload as finalize_private_upload;
pub use bity_ic_storage_canister_api::updates::init_upload as init_private_upload;
pub use bity_ic_storage_canister_api::updates::store_chunk as store_private_chunk;

pub mod derive_vetkey_public_key {
    use candid::CandidType;
    use serde::{Deserialize, Serialize};
    use serde_bytes::ByteBuf;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub context: ByteBuf,
    }

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
        pub context: ByteBuf,
        pub input: ByteBuf,
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
    use candid::Principal;
    use core_nft_common::types::private_content::EncryptionMode;
    use core_nft_common::ReaderInfo;
    use core_nft_common::Sha256Hash;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub hash: Sha256Hash,
        pub entry_name: String,
        pub default_readers: HashMap<Principal, ReaderInfo>,
        pub storage_canister_id: Principal,
        pub storage_path: String,
        pub plaintext_size: u64,
        pub expected_chunks: usize,
        pub chunk_size: Option<u64>,
        pub total_size: u64,
        pub encryption_mode: EncryptionMode,
    }

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum InitPrivateContentUploadError {
        ConcurrentManagementCall,
        AlreadyExists,
        ContentTooLarge,
        InvalidChunkSize,
        StorageCanisterError(String),
    }

    pub type Response = Result<(), InitPrivateContentUploadError>;
}

use core_nft_common::types::init_upload;
impl From<init_upload::InitUploadError>
    for init_private_content_upload::InitPrivateContentUploadError
{
    fn from(error: init_upload::InitUploadError) -> Self {
        match error {
            init_upload::InitUploadError::ConcurrentManagementCall => {
                init_private_content_upload::InitPrivateContentUploadError::ConcurrentManagementCall
            }
            init_upload::InitUploadError::FileAlreadyExists => {
                init_private_content_upload::InitPrivateContentUploadError::AlreadyExists
            }
            init_upload::InitUploadError::StorageCanisterError(msg) => {
                init_private_content_upload::InitPrivateContentUploadError::StorageCanisterError(
                    msg,
                )
            }
        }
    }
}

pub mod store_private_content_chunk {
    use candid::CandidType;
    use candid::Nat;
    use core_nft_common::Sha256Hash;
    use serde::{Deserialize, Serialize};
    use serde_bytes::ByteBuf;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub hash: Sha256Hash,
        pub entry_name: String,
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
    use core_nft_common::Sha256Hash;
    use serde::{Deserialize, Serialize};

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub hash: Sha256Hash,
        pub entry_name: String,
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
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
    pub struct Args {
        pub entry_name: String,
    }

    pub type Response = Result<(), CancelPrivateContentUploadError>;

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum CancelPrivateContentUploadError {
        ConcurrentManagementCall,
        UploadNotInitialized,
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
            cancel_upload::CancelUploadError::StorageCanisterError(msg) => {
                cancel_private_content_upload::CancelPrivateContentUploadError::StorageCanisterError(msg)
            }
        }
    }
}

// 1. From init_private_content_upload::Args -> init_upload::Args
impl From<init_private_content_upload::Args> for init_upload::Args {
    fn from(args: init_private_content_upload::Args) -> Self {
        init_upload::Args {
            file_path: args.entry_name,
            file_hash: hex::encode(args.hash), // FIXME
            file_size: args.plaintext_size,
            chunk_size: args.chunk_size,
        }
    }
}

// 2. From store_private_content_chunk::Args -> store_chunk::Args
impl From<store_private_content_chunk::Args> for store_chunk::Args {
    fn from(args: store_private_content_chunk::Args) -> Self {
        store_chunk::Args {
            chunk_id: args.chunk_index,
            file_path: args.entry_name,
            chunk_data: args.chunk_data.to_vec(),
        }
    }
}

// 3. From finalize_private_content_upload::Args -> finalize_upload::Args
impl From<finalize_private_content_upload::Args> for finalize_upload::Args {
    fn from(args: finalize_private_content_upload::Args) -> Self {
        finalize_upload::Args {
            file_path: args.entry_name,
        }
    }
}

// 4. From cancel_private_content_upload::Args -> cancel_upload::Args
impl From<cancel_private_content_upload::Args> for cancel_upload::Args {
    fn from(args: cancel_private_content_upload::Args) -> Self {
        cancel_upload::Args {
            file_path: args.entry_name,
        }
    }
}
