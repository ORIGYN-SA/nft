use candid::CandidType;
use ic_cdk::api::call::CallResult as Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct Args {
    pub file_path: String,
}

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct FinalizeUploadResp {
    pub url: String,
}

pub type Response = Result<FinalizeUploadResp>;
