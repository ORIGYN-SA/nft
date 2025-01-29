use candid::CandidType;
use serde::{ Deserialize, Serialize };
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;

#[derive(Serialize, Deserialize, CandidType)]
pub struct UpdateDataRequest {
    pub data: Value,
    pub hash_id: String,
}

#[derive(Serialize, Deserialize, CandidType)]
pub struct UpdateDataResponse {
    pub hash_id: String,
    pub previous_data_value: Value,
}
