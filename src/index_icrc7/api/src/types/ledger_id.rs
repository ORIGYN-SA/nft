use candid::CandidType;
use candid::Principal;
use serde::{Deserialize, Serialize};

pub type Args = ();

#[derive(CandidType, Deserialize, Serialize)]
pub struct Response {
    pub ledger_id: Principal,
}
