use crate::guards::caller_is_governance_principal;
use crate::state::mutate_state;
use ic_cdk::{ api::call::RejectionCode, update };
pub use storage_api_canister::updates::{ insert_data, remove_data, update_data };

#[update(guard = "caller_is_governance_principal")]
pub fn insert_data(data: insert_data::InsertDataRequest) -> insert_data::InsertDataResponse {
    match mutate_state(|state| state.data.insert_data(data.data, data.data_id, data.nft_id)) {
        Ok(hash_id) => Ok(insert_data::InsertDataResp { hash_id: hash_id }),
        Err(e) => Err((RejectionCode::CanisterError, e)),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn update_data(data: update_data::UpdateDataRequest) -> update_data::UpdateDataResponse {
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
pub fn remove_data(data: remove_data::RemoveDataRequest) -> remove_data::RemoveDataResponse {
    match mutate_state(|state| state.data.remove_data(data.hash_id)) {
        Ok(previous_data) =>
            Ok(remove_data::RemoveDataResp {
                previous_data_value: previous_data,
            }),
        Err(e) => Err((RejectionCode::CanisterError, e)),
    }
}
