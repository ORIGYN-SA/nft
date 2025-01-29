use candid::{ Encode, Nat, Principal };
use gldt_swap_api_archive::lifecycle::Args as ArgsArchive;
use gldt_swap_api_archive::{ init::InitArgs, post_upgrade::UpgradeArgs };
use gldt_swap_archive_c2c_client::get_archive_size;
use gldt_swap_common::archive::ArchiveCanister;
use gldt_swap_common::swap::NewArchiveError;
use ic_cdk::api::management_canister::main::{
    canister_status,
    create_canister,
    install_code,
    start_canister,
    stop_canister,
    CanisterId,
    CanisterIdRecord,
    CanisterInstallMode,
    CanisterSettings,
    CreateCanisterArgument,
    InstallCodeArgument,
    LogVisibility,
};
use tracing::{ debug, info };
use utils::{ env::Environment, retry_async::retry_async };

use crate::state::{ mutate_state, read_state };
use crate::utils::trace;

const ARCHIVE_WASM: &[u8] = include_bytes!("../../archive/wasm/gldt_swap_archive_canister.wasm.gz");

pub async fn check_storage_and_create_archive() -> Result<(), ()> {
    // check if the capacity is
    if
        let Some(current_archive) = read_state(|s|
            s.data.swaps.get_archive_canisters().last().cloned()
        )
    {
        if is_archive_canister_at_threshold(&current_archive).await {
            info!("ARCHIVE :: at capacity :: creating new archive canister");
            trace("///// Checking archive 3");
            let archive_principal = match create_archive_canister().await {
                Ok(principal) => principal,
                Err(e) => {
                    debug!("{e:?}");
                    mutate_state(|s| {
                        s.data.new_archive_error = Some(e);
                    });
                    return Err(());
                }
            };
            let current_swap_index = read_state(|s| s.data.swaps.get_current_swap_index());
            let archive_buffer = read_state(|s| s.data.archive_buffer);
            let future_swap_index = current_swap_index + Nat::from(archive_buffer);
            mutate_state(|s| {
                s.data.swaps.set_new_archive_canister(ArchiveCanister {
                    canister_id: archive_principal,
                    start_index: future_swap_index,
                    end_index: None,
                    active: false,
                })
            });
            // new archive created
            return Ok(());
        } else {
            // we still have room
            return Ok(());
        }
    }
    // no archive canisters
    Err(())
}

pub async fn create_archive_canister() -> Result<Principal, NewArchiveError> {
    let this_canister_id = read_state(|s| s.env.canister_id());
    let test_mode = read_state(|s| s.env.is_test_mode());
    let mut controllers = get_canister_controllers(this_canister_id).await?;
    controllers.push(ic_cdk::api::id());

    let initial_cycles = if test_mode {
        2_000_000_000_000u64 // 2 Trillion cycles
    } else {
        10_000_000_000_000u64 // 2 Trillion cycles
    };

    let reserved_cycles = if test_mode {
        2_000_000_000_000u64 // 2 Trillion cycles
    } else {
        4_000_000_000_000u64 // 2 Trillion cycles
    };
    // Define the initial settings for the new canister
    let settings = CanisterSettings {
        controllers: Some(controllers), // Ensure the current canister is a controller
        compute_allocation: None,
        memory_allocation: None,
        freezing_threshold: None,
        reserved_cycles_limit: Some(Nat::from(reserved_cycles)),
        log_visibility: Some(LogVisibility::Public),
        wasm_memory_limit: None, // use default of 3GB
    };
    // Step 1: Create the canister
    let canister_id = match
        retry_async(|| {
            create_canister(
                CreateCanisterArgument {
                    settings: Some(settings.clone()),
                },
                initial_cycles as u128
            )
        }, 3).await
    {
        Ok(canister) => canister.0.canister_id,
        Err(e) => {
            return Err(NewArchiveError::CreateCanisterError(format!("{e:?}")));
        }
    };
    let mut current_auth_prins = read_state(|s| s.data.authorized_principals.clone());
    let test_mode = read_state(|s| s.env.is_test_mode());
    let commit_hash = read_state(|s| s.env.commit_hash().to_string());
    current_auth_prins.push(this_canister_id);

    let init_args = match
        Encode!(
            &ArgsArchive::Init(InitArgs {
                commit_hash,
                authorized_principals: current_auth_prins,
                test_mode,
            })
        )
    {
        Ok(encoded_init_args) => encoded_init_args,
        Err(e) => {
            return Err(NewArchiveError::FailedToSerializeInitArgs(format!("{e}")));
        }
    };

    // Step 2: Install the Wasm module to the newly created canister
    let install_args = InstallCodeArgument {
        mode: CanisterInstallMode::Install,
        canister_id: canister_id,
        wasm_module: ARCHIVE_WASM.to_vec(),
        arg: init_args,
    };

    match retry_async(|| install_code(install_args.clone()), 3).await {
        Ok(_) => {
            mutate_state(|s| {
                s.data.new_archive_error = None;
            });
            Ok(canister_id)
        }
        Err(e) => {
            return Err(NewArchiveError::InstallCodeError(format!("{e:?}")));
        }
    }
}

pub async fn is_archive_canister_at_threshold(archive: &ArchiveCanister) -> bool {
    let res = retry_async(|| get_archive_size(archive.canister_id, &()), 3).await;
    let max_canister_archive_threshold = read_state(|s|
        s.data.max_canister_archive_threshold.clone()
    );
    let archive_id = archive.canister_id;
    trace(
        &format!(
            "///// Checking archive 4 : {archive_id:?}. archive size {res:?}. max allowed size : {max_canister_archive_threshold}"
        )
    );
    match res {
        Ok(size) => (size as u128) >= max_canister_archive_threshold,
        Err(_) => false,
    }
}

pub async fn update_archive_canisters() -> Result<(), Vec<String>> {
    let archive_canisters = read_state(|s| s.data.swaps.get_archive_canisters());
    let commit_hash = read_state(|s| s.env.commit_hash().to_string());
    let version = read_state(|s| s.env.version());
    let mut current_auth_prins = read_state(|s| s.data.authorized_principals.clone());
    let this_canister_id = read_state(|s| s.env.canister_id());
    current_auth_prins.push(this_canister_id);

    let init_args = match
        Encode!(
            &ArgsArchive::Upgrade(UpgradeArgs {
                commit_hash,
                version,
            })
        )
    {
        Ok(encoded_init_args) => encoded_init_args,
        Err(e) => {
            return Err(vec![format!("ERROR : failed to create init args with error - {e}")]);
        }
    };

    let mut canister_upgrade_errors = vec![];

    for archive in archive_canisters {
        match
            retry_async(|| {
                stop_canister(CanisterIdRecord {
                    canister_id: archive.canister_id,
                })
            }, 3).await
        {
            Ok(_) => {}
            Err(e) => {
                canister_upgrade_errors.push(
                    format!(
                        "ERROR: archive upgrade :: archive with principal : {} failed to stop with error {:?}",
                        archive.canister_id,
                        e
                    )
                );
                continue;
            }
        }

        let result = {
            let init_args = init_args.clone();
            let wasm_module = ARCHIVE_WASM.to_vec();

            let install_args = InstallCodeArgument {
                mode: CanisterInstallMode::Upgrade(None),
                canister_id: archive.canister_id,
                wasm_module,
                arg: init_args,
            };
            retry_async(|| install_code(install_args.clone()), 3).await
        };

        match result {
            Ok(_) => {
                match
                    retry_async(|| {
                        start_canister(CanisterIdRecord {
                            canister_id: archive.canister_id,
                        })
                    }, 3).await
                {
                    Ok(_) => {}
                    Err(e) => {
                        canister_upgrade_errors.push(
                            format!(
                                "ERROR: archive upgrade :: archive with principal : {} failed to start with error {:?}",
                                archive.canister_id,
                                e
                            )
                        );
                    }
                }
            }
            Err(e) => {
                canister_upgrade_errors.push(
                    format!(
                        "ERROR: archive upgrade :: archive with principal : {} failed to install upgrade {:?}",
                        archive.canister_id,
                        e
                    )
                );
            }
        }
    }

    if canister_upgrade_errors.len() > 0 {
        return Err(canister_upgrade_errors);
    } else {
        Ok(())
    }
}

async fn get_canister_controllers(
    canister_id: CanisterId
) -> Result<Vec<Principal>, NewArchiveError> {
    match retry_async(|| canister_status(CanisterIdRecord { canister_id }), 3).await {
        Ok(res) => Ok(res.0.settings.controllers),
        Err(e) => Err(NewArchiveError::CantFindControllers(format!("{e:?}"))),
    }
}
