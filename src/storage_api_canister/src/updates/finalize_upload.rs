use candid::{ CandidType, Nat };
use serde::{ Deserialize, Serialize };
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use crate::types::value_custom::CustomValue as Value;
use ic_cdk::api::call::CallResult as Result;

#[derive(Serialize, Deserialize, CandidType)]
pub struct Args {
    pub media_hash_id: String,
}

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct FinalizeUploadResp {}

pub type Response = Result<FinalizeUploadResp>;
