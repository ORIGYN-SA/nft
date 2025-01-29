use candid::{ Encode, Nat, Principal };
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
use utils::{ env::Environment, retry_async::retry_async };
use storage_api_canister::lifecycle::{
    Args as ArgsStorage,
    init::InitArgs,
    post_upgrade::UpgradeArgs,
};
use storage_canister_c2c_client::get_storage_size;

use crate::state::read_state;

const STORAGE_WASM: &[u8] = include_bytes!(
    "../../storage_canister/wasm/storage_canister_canister.wasm.gz"
);

pub enum NewStorageError {
    CreateCanisterError(String),
    InstallCodeError(String),
    FailedToSerializeInitArgs(String),
    CantFindControllers(String),
}

pub async fn create_storage_canister() -> Result<Principal, NewStorageError> {
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
            return Err(NewStorageError::CreateCanisterError(format!("{e:?}")));
        }
    };

    let mut current_auth_prins = read_state(|s| s.data.authorized_principals.clone());
    let test_mode = read_state(|s| s.env.is_test_mode());
    let commit_hash = read_state(|s| s.env.commit_hash().to_string());
    current_auth_prins.push(this_canister_id);

    let init_args = match
        Encode!(
            &ArgsStorage::Init(InitArgs {
                commit_hash,
                authorized_principals: current_auth_prins,
                test_mode,
            })
        )
    {
        Ok(encoded_init_args) => encoded_init_args,
        Err(e) => {
            return Err(NewStorageError::FailedToSerializeInitArgs(format!("{e}")));
        }
    };

    // Step 2: Install the Wasm module to the newly created canister
    let install_args = InstallCodeArgument {
        mode: CanisterInstallMode::Install,
        canister_id: canister_id,
        wasm_module: STORAGE_WASM.to_vec(),
        arg: init_args,
    };

    match retry_async(|| install_code(install_args.clone()), 3).await {
        Ok(_) => { Ok(canister_id) }
        Err(e) => {
            return Err(NewStorageError::InstallCodeError(format!("{e:?}")));
        }
    }
}

pub async fn is_storage_canister_at_threshold(storage: Principal) -> bool {
    let res = retry_async(|| get_storage_size(storage, &()), 3).await;
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
            let wasm_module = STORAGE_WASM.to_vec();

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
