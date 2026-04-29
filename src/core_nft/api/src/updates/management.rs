use crate::types::management;

pub use management::update_collection_metadata::{Args as UpdateCollectionMetadataArgs, Response as UpdateCollectionMetadataResponse};
pub use management::mint::{Args as MintArgs, Response as MintResponse};
pub use management::update_nft_metadata::{Args as UpdateNftMetadataArgs, Response as UpdateNftMetadataResponse};
pub use management::burn_nft::{Response as BurnNftResponse};
pub use management::grant_permission::{Args as GrantPermissionArgs, Response as GrantPermissionResponse};
pub use management::revoke_permission::{Args as RevokePermissionArgs, Response as RevokePermissionResponse};
pub use management::get_user_permissions::{Args as GetUserPermissionsArgs, Response as GetUserPermissionsResponse};
pub use management::has_permission::{Args as HasPermissionArgs, Response as HasPermissionResponse};
pub use management::get_upload_status::{Response as GetUploadStatusResponse};
pub use management::get_all_uploads::{Response as GetAllUploadsResponse};

// Re-export storage canister functions
pub use bity_ic_storage_canister_api::updates::init_upload;
pub use bity_ic_storage_canister_api::updates::store_chunk;
pub use bity_ic_storage_canister_api::updates::finalize_upload;
pub use bity_ic_storage_canister_api::updates::cancel_upload;