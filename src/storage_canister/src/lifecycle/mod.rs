pub mod init;
pub mod post_upgrade;
pub mod pre_upgrade;

use crate::state::{ init_state, RuntimeState };

pub fn init_canister(runtime_state: RuntimeState) {
    init_state(runtime_state);
}

use candid::CandidType;
use serde::{ Deserialize, Serialize };

use storage_api_canister::lifecycle::init::InitArgs;
use storage_api_canister::lifecycle::post_upgrade::UpgradeArgs;

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub enum Args {
    Init(InitArgs),
    Upgrade(UpgradeArgs),
}
