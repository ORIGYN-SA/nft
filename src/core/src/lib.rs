use ic_cdk::export_candid;

use crate::state::take_state;
use crate::state::RuntimeState;
use crate::types::icrc7;

mod guards;
mod lifecycle;
mod memory;
mod queries;
mod utils;
// mod migrations;

mod state;
pub mod types;
pub mod updates;

use lifecycle::*;
use queries::*;
use updates::*;

export_candid!();
