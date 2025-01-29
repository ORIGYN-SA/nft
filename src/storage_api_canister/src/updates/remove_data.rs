use candid::CandidType;
use serde::{ Deserialize, Serialize };
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use crate::types::value_custom::CustomValue as Value;

#[derive(Serialize, Deserialize, CandidType)]
pub struct RemoveDataRequest {
    pub hash_id: String,
}

#[derive(Serialize, Deserialize, CandidType)]
pub struct RemoveDataResponse {
    pub previous_data_value: Value,
}
