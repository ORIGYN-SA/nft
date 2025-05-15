use candid::{CandidType, Nat};
use ic_cdk::api::call::CallResult as Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct Args {
    pub file_path: String,
    pub chunk_id: Nat,
    pub chunk_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct StoreChunkResp {}

pub type Response = Result<StoreChunkResp>;
