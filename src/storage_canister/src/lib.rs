use ic_cdk::export_candid;

mod guards;
mod lifecycle;
mod memory;
// mod migrations;

pub mod queries;
mod state;
pub mod types;
pub mod updates;

use lifecycle::*;
use queries::*;
use updates::*;

export_candid!();
