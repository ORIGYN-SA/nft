use ic_cdk::update;
use crate::guards::caller_is_governance_principal;
use crate::state::read_state;
pub use storage_api_canister::queries::get_data;

#[update(guard = "caller_is_governance_principal")]
pub fn get_data(data: get_data::GetDataRequest) -> Result<get_data::GetDataResponse, String> {
    match read_state(|state| { state.data.get_data(data.hash_id) }) {
        Ok(data_value) =>
            Ok(get_data::GetDataResponse {
                data_value: data_value,
            }),
        Err(e) => Err(e),
    }
}
