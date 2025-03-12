use crate::state::read_state;
use ic_cdk::query;
pub use storage_api_canister::queries::get_storage_size::{Args, Response};

#[query]
async fn get_storage_size(_: Args) -> Response {
    read_state(|s| s.data.storage.get_storage_size_bytes())
}
