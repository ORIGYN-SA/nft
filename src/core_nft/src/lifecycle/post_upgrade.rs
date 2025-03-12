use crate::lifecycle::init_canister;
use crate::memory::get_upgrades_memory;
use crate::state::RuntimeState;
use crate::Args;
use candid::{CandidType, Principal};
use canfund::{
    manager::{
        options::{CyclesThreshold, FundManagerOptions, FundStrategy},
        RegisterOpts,
    },
    operations::fetch::FetchCyclesBalanceFromCanisterStatus,
    FundManager,
};
use canister_logger::LogEntry;
use canister_tracing_macros::trace;
use ic_cdk_macros::post_upgrade;
use serde::{Deserialize, Serialize};
use stable_memory::get_reader;
use std::sync::Arc;
use subcanister_manager::SubCanister;
use tracing::info;
use types::BuildVersion;

//todo gwojda add fund_manager in state with skip serialize
fn initialize(canister_id_lst: Vec<Principal>) {
    let mut fund_manager = FundManager::new();

    let funding_config = FundManagerOptions::new()
        .with_interval_secs(12 * 60 * 60)
        .with_strategy(FundStrategy::BelowThreshold(
            CyclesThreshold::new()
                .with_min_cycles(125_000_000_000)
                .with_fund_cycles(250_000_000_000),
        ));

    fund_manager.with_options(funding_config);

    for canister_id in canister_id_lst {
        fund_manager.register(
            canister_id,
            RegisterOpts::new()
                .with_cycles_fetcher(Arc::new(FetchCyclesBalanceFromCanisterStatus::new())),
        );
    }

    fund_manager.start();
}
#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct UpgradeArgs {
    pub version: BuildVersion,
    pub commit_hash: String,
}

#[post_upgrade]
#[trace]
fn post_upgrade(args: Args) {
    match args {
        Args::Init(_) =>
            panic!(
                "Cannot upgrade the canister with an Init argument. Please provide an Upgrade argument."
            ),
        Args::Upgrade(upgrade_args) => {
            let memory = get_upgrades_memory();
            let reader = get_reader(&memory);

            // uncomment these lines if you want to do a normal upgrade
            let (mut state, logs, traces): (RuntimeState, Vec<LogEntry>, Vec<LogEntry>) = serializer
                ::deserialize(reader)
                .unwrap();

            // uncomment these lines if you want to do an upgrade with migration
            // let (runtime_state_v0, logs, traces): (
            //     RuntimeStateV0,
            //     Vec<LogEntry>,
            //     Vec<LogEntry>,
            // ) = serializer::deserialize(reader).unwrap();
            // let mut state = RuntimeState::from(runtime_state_v0);

            state.env.set_version(upgrade_args.version);
            state.env.set_commit_hash(upgrade_args.commit_hash);

            canister_logger::init_with_logs(state.env.is_test_mode(), logs, traces);
            init_canister(state.clone());

            initialize(state.data.sub_canister_manager.list_canisters_ids());

            info!(version = %upgrade_args.version, "Post-upgrade complete");
        }
    }
}
