use canister_client::generate_candid_c2c_call;
use storage_api_canister::get_data::{ Args as GetDataArgs, Response as GetDataResponse };
use storage_api_canister::get_storage_size::{
    Args as GetStorageSizeArgs,
    Response as GetStorageSizeResponse,
};
use storage_api_canister::insert_data::{ Args as InsertDataArgs, Response as InsertDataResponse };
use storage_api_canister::init_upload;
use storage_api_canister::store_chunk;
use storage_api_canister::finalize_upload;
use storage_api_canister::cancel_upload;
use storage_api_canister::delete_file;

pub mod get_storage_size {
    use super::*;
    pub type Args = GetStorageSizeArgs;
    pub type Response = GetStorageSizeResponse;
}

pub mod insert_data {
    use super::*;
    pub type Args = InsertDataArgs;
    pub type Response = InsertDataResponse;
}

pub mod get_data {
    use super::*;
    pub type Args = GetDataArgs;
    pub type Response = GetDataResponse;
}

generate_candid_c2c_call!(get_data);
generate_candid_c2c_call!(insert_data);
generate_candid_c2c_call!(get_storage_size);
generate_candid_c2c_call!(init_upload);
generate_candid_c2c_call!(store_chunk);
generate_candid_c2c_call!(finalize_upload);
generate_candid_c2c_call!(cancel_upload);
generate_candid_c2c_call!(delete_file);
