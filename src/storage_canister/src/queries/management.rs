use crate::state::read_state;
use ic_cdk::query;
pub use storage_api_canister::queries::get_storage_size::{
    GetStorageSizeArgs, GetStorageSizeResponse,
};

#[query]
async fn get_storage_size(_: GetStorageSizeArgs) -> GetStorageSizeResponse {
    read_state(|s| s.data.storage.get_storage_size_bytes())
}
