use canister_client::generate_candid_c2c_call;
use storage_api_canister::get_data::{GetDataRequest, GetDataResponse};
use storage_api_canister::get_storage_size::{GetStorageSizeArgs, GetStorageSizeResponse};
use storage_api_canister::insert_data::{InsertDataRequest, InsertDataResponse};

pub mod get_storage_size {
    use super::*;
    pub type Args = GetStorageSizeArgs;
    pub type Response = GetStorageSizeResponse;
}

pub mod insert_data {
    use super::*;
    pub type Args = InsertDataRequest;
    pub type Response = InsertDataResponse;
}

pub mod get_data {
    use super::*;
    pub type Args = GetDataRequest;
    pub type Response = GetDataResponse;
}

generate_candid_c2c_call!(get_data);
generate_candid_c2c_call!(insert_data);
generate_candid_c2c_call!(get_storage_size);
