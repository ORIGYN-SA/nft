pub mod init;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use init::{InitArgs, UpgradeArgs};

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub enum Args {
    Init(InitArgs),
    Upgrade(UpgradeArgs),
}

pub use init::*;