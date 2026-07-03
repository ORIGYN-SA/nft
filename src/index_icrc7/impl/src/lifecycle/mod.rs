use crate::jobs::start_job;
use crate::state::{init_state, RuntimeState};
pub use candid::Principal;

pub mod init;
mod post_upgrade;
mod pre_upgrade;

pub use init::*;

pub fn init_canister(runtime_state: RuntimeState) {
    init_state(runtime_state);
    start_job();
}
