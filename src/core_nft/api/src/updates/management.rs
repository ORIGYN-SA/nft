use core_nft_common::types::management;

pub use management::burn_nft::Response as BurnNftResponse;
pub use management::get_all_uploads::Response as GetAllUploadsResponse;
pub use management::get_upload_status::Response as GetUploadStatusResponse;
pub use management::get_user_permissions::{
    Args as GetUserPermissionsArgs, Response as GetUserPermissionsResponse,
};
pub use management::grant_permission::{
    Args as GrantPermissionArgs, Response as GrantPermissionResponse,
};
pub use management::has_permission::{
    Args as HasPermissionArgs, Response as HasPermissionResponse,
};
pub use management::mint::{Args as MintArgs, Response as MintResponse};
pub use management::revoke_permission::{
    Args as RevokePermissionArgs, Response as RevokePermissionResponse,
};
pub use management::update_collection_metadata::{
    Args as UpdateCollectionMetadataArgs, Response as UpdateCollectionMetadataResponse,
};
pub use management::update_nft_metadata::{
    Args as UpdateNftMetadataArgs, Response as UpdateNftMetadataResponse,
};

pub use management::cancel_upload::{Args as CancelUploadArgs, Response as CancelUploadResponse};
pub use management::finalize_upload::{
    Args as FinalizeUploadArgs, Response as FinalizeUploadResponse,
};
pub use management::init_upload::{Args as InitUploadArgs, Response as InitUploadResponse};
pub use management::store_chunk::{Args as StoreChunkArgs, Response as StoreChunkResponse};
