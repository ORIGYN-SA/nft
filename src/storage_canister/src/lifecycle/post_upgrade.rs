use crate::memory::get_upgrades_memory;
use crate::state::RuntimeState;
use crate::types::http::certify_all_assets;
use crate::{lifecycle::init_canister, utils::trace};
use candid::CandidType;
use canister_logger::LogEntry;
use canister_tracing_macros::trace;
use ic_cdk_macros::post_upgrade;
use serde::{Deserialize, Serialize};
use stable_memory::get_reader;
use storage_api_canister::Args;
use tracing::info;
use types::BuildVersion;

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
            trace("Post-upgrade started");
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
            init_canister(state);
            // certify_all_assets();

            info!(version = %upgrade_args.version, "Post-upgrade complete");
            trace("Post-upgrade complete");
        }
    }
}
