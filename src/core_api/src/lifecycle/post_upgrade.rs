use candid::CandidType;
use serde::{Deserialize, Serialize};
use types::BuildVersion;

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
pub struct UpgradeArgs {
    pub version: BuildVersion,
    pub commit_hash: String,
}
