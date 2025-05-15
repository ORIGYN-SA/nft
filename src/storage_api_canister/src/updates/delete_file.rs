use candid::CandidType;
use serde::{Deserialize, Serialize};
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use ic_cdk::api::call::CallResult as Result;

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct Args {
    pub file_path: String,
}

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct DeleteFileResp {}

pub type Response = Result<DeleteFileResp>;
