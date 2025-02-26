use candid::{ CandidType, Nat };
use serde::{ Deserialize, Serialize };
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use crate::types::value_custom::CustomValue as Value;
use ic_cdk::api::call::CallResult as Result;

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct Args {
    pub media_hash_id: String,
    pub chunk_id: Nat,
    pub chunk_data: Vec<u8>,
}

#[derive(Serialize, Deserialize, CandidType, Debug)]
pub struct StoreChunkResp {}

pub type Response = Result<StoreChunkResp>;
