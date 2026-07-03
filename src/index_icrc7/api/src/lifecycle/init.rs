use bity_ic_types::BuildVersion;
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct InitArgs {
    pub test_mode: bool,
    pub version: BuildVersion,
    pub commit_hash: String,
    pub authorized_principals: Vec<Principal>,
    pub ledger_canister_id: Principal,
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct UpgradeArgs {
    pub version: BuildVersion,
    pub commit_hash: String,
}
