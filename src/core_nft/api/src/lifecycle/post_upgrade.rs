use candid::CandidType;
use serde::{Deserialize, Serialize};
use bity_ic_types::BuildVersion;

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct UpgradeArgs {
    pub version: BuildVersion,
    pub commit_hash: String,
}