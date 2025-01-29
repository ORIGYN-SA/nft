use crate::state::read_state;
pub use storage_api_canister::queries::get_storage_size::{
    GetStorageSizeArgs,
    GetStorageSizeResponse,
};
use ic_cdk::query;

#[query]
async fn get_storage_size(_: GetStorageSizeArgs) -> GetStorageSizeResponse {
    read_state(|s| s.data.storage.get_storage_size_bytes())
}
