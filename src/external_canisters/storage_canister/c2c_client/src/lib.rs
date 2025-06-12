use bity_ic_canister_client::generate_candid_c2c_call;
use storage_api_canister::cancel_upload;
use storage_api_canister::finalize_upload;
use storage_api_canister::get_storage_size::{
    Args as GetStorageSizeArgs, Response as GetStorageSizeResponse,
};
use storage_api_canister::init_upload;
use storage_api_canister::store_chunk;

pub mod get_storage_size {
    use super::*;
    pub type Args = GetStorageSizeArgs;
    pub type Response = GetStorageSizeResponse;
}

generate_candid_c2c_call!(get_storage_size);
generate_candid_c2c_call!(init_upload);
generate_candid_c2c_call!(store_chunk);
generate_candid_c2c_call!(finalize_upload);
generate_candid_c2c_call!(cancel_upload);
