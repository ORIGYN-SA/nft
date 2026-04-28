pub mod init;
pub mod post_upgrade;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use init::InitArgs;
use post_upgrade::UpgradeArgs;

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub enum Args {
    Init(InitArgs),
    Upgrade(UpgradeArgs),
}

pub use init::*;
pub use post_upgrade::*;