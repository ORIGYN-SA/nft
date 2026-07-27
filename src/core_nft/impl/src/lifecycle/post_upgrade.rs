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
use core_nft_common::types::http::{add_redirection, certify_all_assets};
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

            state.data.sub_canister_manager.sub_canister_manager.wasm = STORAGE_WASM.to_vec();
            // funding_config is #[serde(skip)]: without this, canfund falls back to
            // its defaults (daily / 250B threshold) after the upgrade.
            state
                .data
                .sub_canister_manager
                .sub_canister_manager
                .funding_config = default_funding_config();

            migrate_internal_filestorage_into_public_content(&mut state);

            if let Some(base_url) = upgrade_args.base_url.clone() {
                state.data.base_url = Some(base_url);
                // Existing files keep serving from their old host otherwise;
                // re-render the stored redirects so they move with the domain.
                rerender_media_redirections(&mut state);
            }

            state.env.set_version(upgrade_args.version);
            state.env.set_commit_hash(upgrade_args.commit_hash.clone());

            bity_ic_canister_logger::init_with_logs(state.env.is_test_mode(), logs, traces);
            init_canister(state.clone());
            replace_icrc3(icrc3);
            start_default_archive_job();

            // The certification tree is heap state and does not survive an
            // upgrade. Without re-seeding it, the skip-certification entries for
            // /metrics, /logs and /trace are missing and those endpoints answer
            // 503 "expression path not found in the tree" on the certified
            // domain, which is what every upgraded collection does today.
            certify_all_assets();

            let media_redirections = read_state(|state| state.data.media_redirections.clone());
            for (path, redirection_url) in media_redirections {
                add_redirection(path, redirection_url);
            }

            schedule_storage_fleet_upgrade(upgrade_args.version, upgrade_args.commit_hash.clone());

            info!(version = %upgrade_args.version, "Post-upgrade complete");
        }
    }
}

/// Renders a redirection target from the base_url template, mirroring the URL
/// construction in `finalize_upload`.
fn render_redirection_url(template: &str, canister_id: &str, path: &str) -> String {
    crate::utils::render_media_url(&Some(template.to_string()), false, canister_id, path)
}

/// Re-renders every stored media redirection against the current base_url
/// template so already-uploaded files move with the domain (e.g. switching
/// the serving host from `{canister_id}.icp0.io` to `{canister_id}.raw.icp0.io`).
///
/// The storage canister that holds each file is looked up in the public
/// content system (which includes migrated legacy entries) with a fallback to
/// `internal_filestorage`; entries whose storage canister cannot be resolved
/// are left untouched. The redirects are certified afterwards by the existing
/// `add_redirection` loop in post_upgrade.
fn rerender_media_redirections(state: &mut RuntimeState) {
    let Some(template) = state.data.base_url.clone() else {
        return;
    };

    let paths: Vec<String> = state.data.media_redirections.keys().cloned().collect();
    let mut rendered = 0usize;

    for path in paths {
        // Redirection keys carry a leading slash; the content bookkeeping is
        // keyed by the upload's original file_path, which may not.
        let trimmed = path.trim_start_matches('/');

        let storage_canister = state
            .data
            .public_content_system
            .get_public_file_by_path(&path)
            .or_else(|| {
                state
                    .data
                    .public_content_system
                    .get_public_file_by_path(trimmed)
            })
            .map(|entry| entry.storage_canister_id)
            .or_else(|| state.internal_filestorage.get(&path).map(|d| d.canister))
            .or_else(|| state.internal_filestorage.get(trimmed).map(|d| d.canister));

        if let Some(canister_id) = storage_canister {
            let new_url = render_redirection_url(&template, &canister_id.to_string(), &path);
            state.data.media_redirections.insert(path, new_url);
            rendered += 1;
        }
    }

    if rendered > 0 {
        info!("Re-rendered {rendered} media redirections against the current base_url");
    }
}

#[cfg(test)]
mod tests {
    use super::render_redirection_url;

    #[test]
    fn renders_template_with_canister_id() {
        assert_eq!(
            render_redirection_url(
                "https://{canister_id}.raw.icp0.io",
                "aaaaa-aa",
                "/img/photo.png"
            ),
            "https://aaaaa-aa.raw.icp0.io/img/photo.png"
        );
    }

    #[test]
    fn strips_trailing_slash_from_base() {
        assert_eq!(
            render_redirection_url("https://{canister_id}.raw.icp0.io/", "aaaaa-aa", "/a.png"),
            "https://aaaaa-aa.raw.icp0.io/a.png"
        );
    }

    #[test]
    fn renders_opaque_templates_verbatim() {
        assert_eq!(
            render_redirection_url("test", "aaaaa-aa", "/a.png"),
            "test/a.png"
        );
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
