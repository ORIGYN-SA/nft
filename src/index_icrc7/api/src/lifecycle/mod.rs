pub mod init;

use candid::CandidType;
use init::{InitArgs, UpgradeArgs};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub enum Args {
    Init(InitArgs),
    Upgrade(UpgradeArgs),
}

pub use init::*;
