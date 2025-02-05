use ic_cdk::export_candid;

mod guards;
mod lifecycle;
mod memory;
// mod migrations;

mod state;
pub mod queries;
pub mod types;
pub mod updates;

use lifecycle::*;
use queries::*;
use updates::*;

export_candid!();
