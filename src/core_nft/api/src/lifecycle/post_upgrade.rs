use bity_ic_types::BuildVersion;
use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct UpgradeArgs {
    pub version: BuildVersion,
    pub commit_hash: String,
}
