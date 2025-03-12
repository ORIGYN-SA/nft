use candid::CandidType;
use serde::{Deserialize, Serialize};
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use crate::types::value_custom::CustomValue as Value;
use ic_cdk::api::call::CallResult as Result;

#[derive(Serialize, Deserialize, CandidType)]
pub struct Args {
    pub data: Value,
    pub hash_id: String,
}

#[derive(Serialize, Deserialize, CandidType)]
pub struct UpdateDataResp {
    pub hash_id: String,
    pub previous_data_value: Option<Value>,
}

pub type Response = Result<UpdateDataResp>;
