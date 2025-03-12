use candid::{CandidType, Nat, Principal};
use serde::{Deserialize, Serialize};
use types::BuildVersion;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct InitArgs {
    pub test_mode: bool,
    pub version: BuildVersion,
    pub commit_hash: String,
    pub authorized_principals: Vec<Principal>,
}
