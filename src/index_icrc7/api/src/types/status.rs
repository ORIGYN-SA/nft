use candid::CandidType;
use serde::{Deserialize, Serialize};

pub type Args = ();

#[derive(CandidType, Deserialize, Serialize)]
pub struct Response {
    pub last_block_id: u64,
}
