pub mod init;
mod post_upgrade;
mod pre_upgrade;

use bity_ic_types::BuildVersion;
pub use candid::Principal;
pub use init::*;

use crate::jobs::start_job;
use crate::state::{init_state, RuntimeState};

pub fn init_canister(runtime_state: RuntimeState) {
    init_state(runtime_state);
    start_job();
}

#[derive(candid::CandidType, serde::Serialize, serde::Deserialize, Debug)]

pub enum Args {
    Init(InitArgs),
    Upgrade(UpgradeArgs),
}

#[derive(candid::CandidType, serde::Serialize, serde::Deserialize, Debug)]

pub struct InitArgs {
    pub test_mode: bool,
    pub version: BuildVersion,
    pub commit_hash: String,
    pub authorized_principals: Vec<Principal>,
    pub ledger_canister_id: Principal,
}

#[derive(candid::CandidType, serde::Serialize, serde::Deserialize, Debug)]
pub struct UpgradeArgs {
    pub version: BuildVersion,
    pub commit_hash: String,
}
