use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use crate::types::value_custom::CustomValue as Value;
use ic_cdk::api::call::CallResult as Result;

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct Args {
    pub file_path: String,
    pub file_hash: String,
    pub file_size: u64,
    pub chunk_size: Option<u64>,
}

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct InitUploadResp {}

pub type Response = Result<InitUploadResp>;
