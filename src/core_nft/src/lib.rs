use ic_cdk::export_candid;

use crate::types::icrc7;
use crate::types::management;

mod guards;
pub mod lifecycle;
mod memory;
pub mod queries;
pub mod updates;
mod utils;
mod sub_canister_manager;
// mod migrations;

mod state;
pub mod types;

pub use lifecycle::*;
pub use queries::*;
pub use updates::*;

export_candid!();
