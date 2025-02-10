use candid::CandidType;
use serde::{Deserialize, Serialize};
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use crate::types::value_custom::CustomValue as Value;

#[derive(Serialize, Deserialize, CandidType)]
pub struct GetDataRequest {
    pub hash_id: String,
}

#[derive(Serialize, Deserialize, CandidType)]
pub struct GetDataResponse {
    pub data_value: Value,
}
