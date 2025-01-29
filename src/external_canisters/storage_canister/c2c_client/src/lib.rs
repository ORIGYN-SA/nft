use canister_client::generate_candid_c2c_call;
use storage_api_canister::get_storage_size::{ GetStorageSizeArgs, GetStorageSizeResponse };

pub mod get_storage_size {
    use super::*;
    pub type Args = GetStorageSizeArgs;
    pub type Response = GetStorageSizeResponse;
}

generate_candid_c2c_call!(get_storage_size);
