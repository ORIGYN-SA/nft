use crate::lifecycle::init_canister;
use crate::memory::get_upgrades_memory;
use crate::migrations::migrate_internal_filestorage_into_public_content;
use crate::state::{
    mutate_state, read_state, replace_icrc3, start_default_archive_job, RuntimeState,
};
use bity_ic_canister_logger::LogEntry;
use bity_ic_canister_tracing_macros::trace;
use bity_ic_icrc3::icrc3::ICRC3;
use bity_ic_stable_memory::get_reader;
use bity_ic_types::BuildVersion;
use core_nft_api::lifecycle::Args;
use core_nft_common::types::http::add_redirection;
use core_nft_common::types::sub_canister::default_funding_config;
use ic_cdk_macros::post_upgrade;
use std::time::Duration;
use tracing::{error, info};

const STORAGE_WASM: &[u8] = include_bytes!("../../../../../wasm/storage_canister.wasm.gz");

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

            // Every field added after the first deployed generation carries
            // #[serde(default)] (and the serializer encodes structs as field-name
            // maps), so state written by ANY previous version deserializes here
            // directly and repeated upgrades never reset accumulated state.
            let (mut state, logs, traces, icrc3): (
                RuntimeState,
                Vec<LogEntry>,
                Vec<LogEntry>,
                ICRC3,
            ) = bity_ic_serializer::deserialize(reader).unwrap();

            if let Some(key_name) = upgrade_args.vetkd_key_name.clone() {
                state.data.private_content_system.config.vetkd_key_name = key_name;
            }
            if let Some(context) = upgrade_args.vetkd_context.clone() {
                state.data.private_content_system.config.vetkd_context = context;
            }
            if let Some(base_url) = upgrade_args.base_url.clone() {
                state.data.base_url = Some(base_url);
            }

            state.data.sub_canister_manager.sub_canister_manager.wasm = STORAGE_WASM.to_vec();
            // funding_config is #[serde(skip)]: without this, canfund falls back to
            // its defaults (daily / 250B threshold) after the upgrade.
            state
                .data
                .sub_canister_manager
                .sub_canister_manager
                .funding_config = default_funding_config();

            migrate_internal_filestorage_into_public_content(&mut state);

            state.env.set_version(upgrade_args.version);
            state.env.set_commit_hash(upgrade_args.commit_hash.clone());

            bity_ic_canister_logger::init_with_logs(state.env.is_test_mode(), logs, traces);
            init_canister(state.clone());
            replace_icrc3(icrc3);
            start_default_archive_job();

            let media_redirections = read_state(|state| state.data.media_redirections.clone());
            for (path, redirection_url) in media_redirections {
                add_redirection(path, redirection_url);
            }

            schedule_storage_fleet_upgrade(upgrade_args.version, upgrade_args.commit_hash.clone());

            info!(version = %upgrade_args.version, "Post-upgrade complete");
        }
    }
}

/// Upgrades every existing storage sub-canister to the embedded storage wasm.
/// Inter-canister calls are forbidden inside post_upgrade itself, so this runs
/// on a zero-delay timer right after the upgrade completes.
fn schedule_storage_fleet_upgrade(version: BuildVersion, commit_hash: String) {
    ic_cdk_timers::set_timer(Duration::ZERO, async move {
        let mut manager =
            read_state(|state| state.data.sub_canister_manager.sub_canister_manager.clone());

        if manager.sub_canisters.is_empty() {
            return;
        }

        let result = manager
            .update_canisters(bity_ic_storage_canister_api::lifecycle::Args::Upgrade(
                bity_ic_storage_canister_api::post_upgrade::UpgradeArgs {
                    version,
                    commit_hash,
                },
            ))
            .await;

        mutate_state(|state| {
            state.data.sub_canister_manager.sub_canister_manager = manager;
        });

        match result {
            Ok(()) => info!("Storage canisters upgraded"),
            Err(errors) => {
                for e in errors {
                    error!("Storage canister upgrade failed: {e}");
                }
            }
        }
    });
}
