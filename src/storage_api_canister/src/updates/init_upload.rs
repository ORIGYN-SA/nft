use candid::CandidType;
use ic_cdk::api::call::CallResult as Result;
use serde::{Deserialize, Serialize};

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
