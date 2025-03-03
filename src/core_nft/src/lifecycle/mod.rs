pub mod init;
pub mod post_upgrade;
pub mod pre_upgrade;

use crate::state::{ init_state, RuntimeState };

pub fn init_canister(runtime_state: RuntimeState) {
    init_state(runtime_state);
    crate::jobs::start();
}

use candid::CandidType;
use serde::{ Deserialize, Serialize };

use crate::init::InitArgs;
use crate::post_upgrade::UpgradeArgs;

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub enum Args {
    Init(InitArgs),
    Upgrade(UpgradeArgs),
}
