use candid::CandidType;
use serde::{Deserialize, Serialize};
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use crate::types::value_custom::CustomValue as Value;
use ic_cdk::api::call::CallResult as Result;

#[derive(Serialize, Deserialize, CandidType)]
pub struct Args {
    pub hash_id: String,
}

#[derive(Serialize, Deserialize, CandidType)]
pub struct GetDataResp {
    pub data_value: Value,
}

pub type Response = Result<GetDataResp>;
