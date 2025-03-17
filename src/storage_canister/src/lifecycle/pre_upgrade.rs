use crate::memory::get_upgrades_memory;
use crate::state::take_state;
use bity_ic_canister_tracing_macros::trace;
use bity_ic_stable_memory::get_writer;
use ic_cdk_macros::pre_upgrade;
use tracing::info;

#[pre_upgrade]
#[trace]
fn pre_upgrade() {
    info!("Pre upgrade.");

    let runtime_state = take_state();

    let logs = bity_ic_canister_logger::export_logs();
    let traces = bity_ic_canister_logger::export_traces();

    let stable_state = (runtime_state, logs, traces);

    let mut memory = get_upgrades_memory();
    let writer = get_writer(&mut memory);

    bity_ic_serializer::serialize(stable_state, writer).unwrap();
}
