use candid::{ CandidType, Principal };
use serde::{ Deserialize, Serialize };

#[derive(Deserialize, Serialize, CandidType, Debug, Clone)]
pub struct InitArgs {
    pub test_mode: bool,
    pub commit_hash: String,
    pub authorized_principals: Vec<Principal>,
}
