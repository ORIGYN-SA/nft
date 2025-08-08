use crate::types::permissions::Permission;
use crate::types::value_custom::CustomValue;

use bity_ic_storage_canister_api::types::storage::UploadState;
use candid::{CandidType, Nat, Principal};
use icrc_ledger_types::icrc::generic_value::ICRC3Value;
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod mint {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub token_owner: Account,
        pub memo: Option<serde_bytes::ByteBuf>,
        pub metadata: Vec<(String, ICRC3Value)>,
    }
    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum MintError {
        ConcurrentManagementCall,
        ExceedMaxAllowedSupplyCap,
        TokenAlreadyExists,
        InvalidMemo,
        StorageCanisterError(String),
    }
    pub type Response = Result<Nat, MintError>;
}

pub mod burn_nft {
    use super::*;

    pub type Args = Nat;
    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum BurnNftError {
        NotTokenOwner,
        ConcurrentManagementCall,
        TokenDoesNotExist,
        StorageCanisterError(String),
    }
    pub type Response = Result<(), BurnNftError>;
}

pub mod update_nft_metadata {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub token_id: Nat,
        pub metadata: Vec<(String, ICRC3Value)>,
    }
    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum UpdateNftMetadataError {
        ConcurrentManagementCall,
        TokenDoesNotExist,
        StorageCanisterError(String),
    }
    pub type Response = Result<Nat, UpdateNftMetadataError>;
}
pub mod get_upload_status {
    use super::*;

    pub type Args = String;
    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum GetUploadStatusError {
        UploadNotFound,
        StorageCanisterError(String),
    }
    pub type Response = Result<UploadState, GetUploadStatusError>;
}

pub mod get_all_uploads {
    use super::*;

    pub type Args0 = Option<Nat>;
    pub type Args1 = Option<Nat>;
    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum GetAllUploadsError {
        StorageCanisterError(String),
    }
    pub type Response = Result<HashMap<String, UploadState>, GetAllUploadsError>;
}

pub mod update_collection_metadata {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub description: Option<String>,
        pub symbol: Option<String>,
        pub name: Option<String>,
        pub logo: Option<String>,
        pub supply_cap: Option<Nat>,
        pub max_query_batch_size: Option<Nat>,
        pub max_update_batch_size: Option<Nat>,
        pub max_take_value: Option<Nat>,
        pub default_take_value: Option<Nat>,
        pub max_memo_size: Option<Nat>,
        pub atomic_batch_transfers: Option<bool>,
        pub tx_window: Option<Nat>,
        pub permitted_drift: Option<Nat>,
        pub max_canister_storage_threshold: Option<Nat>,
        pub collection_metadata: Option<HashMap<String, CustomValue>>,
    }
    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum UpdateCollectionMetadataError {
        ConcurrentManagementCall,
        StorageCanisterError(String),
    }
    pub type Response = Result<(), UpdateCollectionMetadataError>;
}

pub mod init_upload {
    use bity_ic_storage_canister_api::updates::init_upload;
    pub use bity_ic_storage_canister_api::updates::init_upload::InitUploadResp;
    use candid::CandidType;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum InitUploadError {
        ConcurrentManagementCall,
        FileAlreadyExists,
        StorageCanisterError(String),
    }

    pub type Args = init_upload::Args;
    pub type Response = Result<init_upload::InitUploadResp, InitUploadError>;

    pub fn from_storage_response(resp: init_upload::Response) -> Response {
        match resp {
            Ok(data) => Ok(data),
            Err(e) => match e {
                init_upload::InitUploadError::FileAlreadyExists => {
                    Err(InitUploadError::FileAlreadyExists)
                }
                _ => Err(InitUploadError::StorageCanisterError(format!("{:?}", e))),
            },
        }
    }
}

pub mod store_chunk {
    use bity_ic_storage_canister_api::updates::store_chunk;
    pub use bity_ic_storage_canister_api::updates::store_chunk::StoreChunkResp;
    use candid::CandidType;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum StoreChunkError {
        ConcurrentManagementCall,
        UploadNotInitialized,
        UploadAlreadyFinalized,
        StorageCanisterError(String),
    }

    pub type Args = store_chunk::Args;
    pub type Response = Result<store_chunk::StoreChunkResp, StoreChunkError>;

    pub fn from_storage_response(resp: store_chunk::Response) -> Response {
        match resp {
            Ok(data) => Ok(data),
            Err(e) => match e {
                store_chunk::StoreChunkError::UploadNotInitialized => {
                    Err(StoreChunkError::UploadNotInitialized)
                }
                store_chunk::StoreChunkError::UploadAlreadyFinalized => {
                    Err(StoreChunkError::UploadAlreadyFinalized)
                }
                _ => Err(StoreChunkError::StorageCanisterError(format!("{:?}", e))),
            },
        }
    }
}

pub mod finalize_upload {
    use bity_ic_storage_canister_api::updates::finalize_upload;
    pub use bity_ic_storage_canister_api::updates::finalize_upload::FinalizeUploadResp;
    use candid::CandidType;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum FinalizeUploadError {
        ConcurrentManagementCall,
        UploadNotStarted,
        UploadAlreadyFinalized,
        IncompleteUpload,
        StorageCanisterError(String),
    }

    pub type Args = finalize_upload::Args;
    pub type Response = Result<finalize_upload::FinalizeUploadResp, FinalizeUploadError>;

    pub fn from_storage_response(resp: finalize_upload::Response) -> Response {
        match resp {
            Ok(data) => Ok(data),
            Err(e) => match e {
                finalize_upload::FinalizeUploadError::UploadNotStarted => {
                    Err(FinalizeUploadError::UploadNotStarted)
                }
                finalize_upload::FinalizeUploadError::UploadAlreadyFinalized => {
                    Err(FinalizeUploadError::UploadAlreadyFinalized)
                }
                finalize_upload::FinalizeUploadError::IncompleteUpload => {
                    Err(FinalizeUploadError::IncompleteUpload)
                }
                _ => Err(FinalizeUploadError::StorageCanisterError(format!(
                    "{:?}",
                    e
                ))),
            },
        }
    }
}

pub mod cancel_upload {
    use bity_ic_storage_canister_api::updates::cancel_upload;
    pub use bity_ic_storage_canister_api::updates::cancel_upload::CancelUploadResp;
    use candid::CandidType;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum CancelUploadError {
        ConcurrentManagementCall,
        UploadNotInitialized,
        UploadAlreadyFinalized,
        StorageCanisterError(String),
    }

    pub type Args = cancel_upload::Args;
    pub type Response = Result<cancel_upload::CancelUploadResp, CancelUploadError>;

    pub fn from_storage_response(resp: cancel_upload::Response) -> Response {
        match resp {
            Ok(data) => Ok(data),
            Err(e) => match e {
                cancel_upload::CancelUploadError::UploadNotInitialized => {
                    Err(CancelUploadError::UploadNotInitialized)
                }
            },
        }
    }
}

pub mod grant_permission {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub principal: Principal,
        pub permission: Permission,
    }
    pub type Response = Result<(), GrantPermissionError>;

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum GrantPermissionError {
        ConcurrentManagementCall,
        DefaultError(String),
    }
}

pub mod revoke_permission {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub principal: Principal,
        pub permission: Permission,
    }
    pub type Response = Result<(), RevokePermissionError>;

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum RevokePermissionError {
        ConcurrentManagementCall,
        DefaultError(String),
    }
}

pub mod get_user_permissions {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub principal: Principal,
    }
    pub type Response = Result<Vec<Permission>, GetUserPermissionsError>;

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum GetUserPermissionsError {
        UserNotFound,
        DefaultError(String),
    }
}

pub mod has_permission {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub principal: Principal,
        pub permission: Permission,
    }
    pub type Response = Result<bool, HasPermissionError>;

    #[derive(Serialize, Deserialize, CandidType, Debug)]
    pub enum HasPermissionError {
        DefaultError(String),
    }
}
