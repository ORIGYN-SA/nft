use ic_cdk::query;

use crate::state::read_state;
pub use crate::types::{ledger_id, status};

#[query]
pub fn status() -> status::Response {
    let last_block_id = read_state(|state| state.data.last_block_id);

    status::Response { last_block_id }
}

#[query]
pub fn ledger_id() -> ledger_id::Response {
    let ledger_id = read_state(|state| state.data.ledger_canister_id);

    ledger_id::Response { ledger_id }
}
