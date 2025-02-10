use candid::{ CandidType, Nat };
use serde::{ Deserialize, Serialize };
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use crate::types::value_custom::CustomValue as Value;
use ic_cdk::api::call::CallResult as Result;

#[derive(Serialize, Deserialize, CandidType)]
pub struct InsertDataRequest {
    pub data: Value,
    pub data_id: String,
    pub nft_id: Option<Nat>,
}

#[derive(Serialize, Deserialize, CandidType)]
pub struct InsertDataResp {
    pub hash_id: String,
}

pub type InsertDataResponse = Result<InsertDataResp>;
