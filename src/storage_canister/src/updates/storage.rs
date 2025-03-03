use crate::guards::caller_is_governance_principal;
use crate::state::mutate_state;
use ic_cdk::{ api::call::RejectionCode, update };
pub use storage_api_canister::cancel_upload;
pub use storage_api_canister::delete_file;
pub use storage_api_canister::init_upload;
pub use storage_api_canister::store_chunk;
pub use storage_api_canister::finalize_upload;
pub use storage_api_canister::updates::{ insert_data, remove_data, update_data };

#[update(guard = "caller_is_governance_principal")]
pub fn insert_data(data: insert_data::Args) -> insert_data::Response {
    match mutate_state(|state| state.data.insert_data(data.data, data.data_id, data.nft_id)) {
        Ok(hash_id) => Ok(insert_data::InsertDataResp { hash_id: hash_id }),
        Err(e) => Err((RejectionCode::CanisterError, e)),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn update_data(data: update_data::Args) -> update_data::Response {
    match mutate_state(|state| state.data.update_data(data.hash_id, data.data)) {
        Ok((hash_id, previous_data)) =>
            Ok(update_data::UpdateDataResp {
                hash_id: hash_id,
                previous_data_value: previous_data,
            }),
        Err(e) => Err((RejectionCode::CanisterError, e)),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn remove_data(data: remove_data::Args) -> remove_data::Response {
    match mutate_state(|state| state.data.remove_data(data.file_path)) {
        Ok(previous_data) =>
            Ok(remove_data::RemoveDataResp {
                previous_data_value: previous_data,
            }),
        Err(e) => Err((RejectionCode::CanisterError, e)),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn init_upload(data: init_upload::Args) -> init_upload::Response {
    match mutate_state(|state| state.data.init_upload(data)) {
        Ok(_) => Ok(init_upload::InitUploadResp {}),
        Err(e) => Err((RejectionCode::CanisterError, e)),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn store_chunk(data: store_chunk::Args) -> store_chunk::Response {
    match mutate_state(|state| state.data.store_chunk(data)) {
        Ok(_) => Ok(store_chunk::StoreChunkResp {}),
        Err(e) => Err((RejectionCode::CanisterError, e)),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn finalize_upload(data: finalize_upload::Args) -> finalize_upload::Response {
    match mutate_state(|state| state.data.finalize_upload(data)) {
        Ok(_) => Ok(finalize_upload::FinalizeUploadResp {}),
        Err(e) => Err((RejectionCode::CanisterError, e)),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn cancel_upload(data: cancel_upload::Args) -> cancel_upload::Response {
    match mutate_state(|state| state.data.cancel_upload(data.file_path)) {
        Ok(_) => Ok(cancel_upload::CancelUploadResp {}),
        Err(e) => Err((RejectionCode::CanisterError, e)),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn delete_file(data: delete_file::Args) -> delete_file::Response {
    match mutate_state(|state| state.data.delete_file(data.file_path)) {
        Ok(_) => Ok(delete_file::DeleteFileResp {}),
        Err(e) => Err((RejectionCode::CanisterError, e)),
    }
}
