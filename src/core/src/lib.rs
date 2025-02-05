use ic_cdk::export_candid;

use candid::Principal;
use crate::types::icrc7;
use crate::types::management;

mod guards;
mod lifecycle;
mod memory;
mod queries;
mod utils;
mod sub_canister_manager;
// mod migrations;

mod state;
pub mod types;
pub mod updates;

use lifecycle::*;
use queries::*;
use updates::*;

export_candid!();
