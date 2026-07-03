pub mod init;
pub mod post_upgrade;

use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::init::InitArgs;
use crate::post_upgrade::UpgradeArgs;

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub enum Args {
    Init(InitArgs),
    Upgrade(UpgradeArgs),
}
