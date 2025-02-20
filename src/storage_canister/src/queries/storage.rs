use crate::guards::caller_is_governance_principal;
use crate::state::read_state;
use ic_cdk::api::call::RejectionCode;
use ic_cdk::update;
pub use storage_api_canister::queries::get_data;

#[update(guard = "caller_is_governance_principal")]
pub fn get_data(data: get_data::Args) -> get_data::Response {
    match read_state(|state| state.data.get_data(data.hash_id)) {
        Ok(data_value) =>
            Ok(get_data::GetDataResp {
                data_value: data_value,
            }),
        Err(e) => Err((RejectionCode::CanisterError, e)),
    }
}
