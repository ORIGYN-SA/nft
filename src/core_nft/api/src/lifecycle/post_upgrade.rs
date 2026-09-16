use bity_ic_types::BuildVersion;
use candid::CandidType;
use core_nft_common::types::sub_canister::StorageCyclesConfig;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct UpgradeArgs {
    pub version: BuildVersion,
    pub commit_hash: String,
    pub vetkd_key_name: Option<String>,
    pub vetkd_context: Option<String>,
    pub base_url: Option<String>,
    /// Replaces the stored cycle settings for storage sub-canisters. `None`
    /// keeps whatever the collection already has.
    pub storage_cycles: Option<StorageCyclesConfig>,
}
