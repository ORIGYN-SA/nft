use ic_cdk::update;
use crate::guards::caller_is_governance_principal;
use crate::state::mutate_state;
pub use storage_api_canister::updates::{ insert_data, update_data, remove_data };

#[update(guard = "caller_is_governance_principal")]
pub fn insert_data(
    data: insert_data::InsertDataRequest
) -> Result<insert_data::InsertDataResponse, String> {
    match mutate_state(|state| { state.data.insert_data(data.data, data.data_id, data.nft_id) }) {
        Ok(hash_id) =>
            Ok(insert_data::InsertDataResponse {
                hash_id: hash_id,
            }),
        Err(e) => Err(e),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn update_data(
    data: update_data::UpdateDataRequest
) -> Result<update_data::UpdateDataResponse, String> {
    match mutate_state(|state| { state.data.update_data(data.hash_id, data.data) }) {
        Ok((hash_id, previous_data)) =>
            Ok(update_data::UpdateDataResponse {
                hash_id: hash_id,
                previous_data_value: previous_data,
            }),
        Err(e) => Err(e),
    }
}

#[update(guard = "caller_is_governance_principal")]
pub fn remove_data(
    data: remove_data::RemoveDataRequest
) -> Result<remove_data::RemoveDataResponse, String> {
    match mutate_state(|state| { state.data.remove_data(data.hash_id) }) {
        Ok(previous_data) =>
            Ok(remove_data::RemoveDataResponse {
                previous_data_value: previous_data,
            }),
        Err(e) => Err(e),
    }
}
