use std::collections::HashMap;

use candid::{ Encode, Nat, Principal };
use ic_cdk::api::management_canister::main::{
    canister_status,
    create_canister,
    install_code,
    start_canister,
    stop_canister,
    CanisterIdRecord,
    CanisterInstallMode,
    CanisterSettings,
    CreateCanisterArgument,
    InstallCodeArgument,
    LogVisibility,
};
use utils::retry_async::retry_async;
use storage_canister_c2c_client::{ get_storage_size, insert_data, get_data };
use crate::utils::trace;
use crate::types::sub_canister;
use serde::{ Deserialize, Serialize };
use storage_api_canister::types::value_custom::CustomValue as Value;
use storage_api_canister::utils::get_value_size;

#[derive(Debug)]
pub enum NewStorageError {
    CreateCanisterError(String),
    InstallCodeError(String),
    FailedToSerializeInitArgs(String),
}

const MAX_STORAGE_SIZE: u128 = 500 * 1024 * 1024 * 1024; // 500 GiB TODO maybe we should put a be less here ?

// TODO

pub trait SubCanister {
    type InitArgs;
    type UpgradeArgs;

    async fn create_canister(&mut self) -> Result<Canister, NewStorageError>;
    async fn update_canisters(&mut self) -> Result<(), Vec<String>>;
    fn list_canisters(&self) -> Vec<Canister>;
    fn list_canisters_ids(&self) -> Vec<Principal>;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SubCanisterManager<I, U> {
    pub init_args: I,
    pub upgrade_args: U,
    pub master_canister_id: Principal,
    pub sub_canisters: HashMap<Principal, Canister>,
    pub controllers: Vec<Principal>,
    pub authorized_principal: Vec<Principal>,
    pub initial_cycles: u128,
    pub reserved_cycles: u128,
    pub test_mode: bool,
    pub commit_hash: String,
    pub wasm: Vec<u8>,
}

impl<I, U> SubCanisterManager<I, U> {
    pub fn new(
        init_args: I,
        upgrade_args: U,
        master_canister_id: Principal,
        sub_canisters: HashMap<Principal, Canister>,
        mut controllers: Vec<Principal>,
        mut authorized_principal: Vec<Principal>,
        initial_cycles: u128,
        reserved_cycles: u128,
        test_mode: bool,
        commit_hash: String,
        wasm: Vec<u8>
    ) -> Self {
        controllers.push(master_canister_id);
        authorized_principal.push(master_canister_id);

        Self {
            init_args,
            upgrade_args,
            master_canister_id,
            sub_canisters,
            controllers,
            authorized_principal,
            initial_cycles,
            reserved_cycles,
            test_mode,
            commit_hash,
            wasm,
        }
    }
}

impl SubCanister for SubCanisterManager<sub_canister::InitArgs, sub_canister::UpgradeArgs> {
    type InitArgs = sub_canister::InitArgs;
    type UpgradeArgs = sub_canister::UpgradeArgs;

    async fn create_canister(&mut self) -> Result<Canister, NewStorageError> {
        // find in self.sub_canisters if a canister is already created but not installed
        let mut canister_id = Principal::anonymous();

        for (_canister_id, canister) in self.sub_canisters.iter() {
            if canister.state == CanisterState::Created {
                canister_id = _canister_id.clone();
                break;
            }
        }
        if canister_id == Principal::anonymous() {
            // Define the initial settings for the new canister
            let settings = CanisterSettings {
                controllers: Some(self.controllers.clone()), // Ensure the current canister is a controller
                compute_allocation: None,
                memory_allocation: None,
                freezing_threshold: None,
                reserved_cycles_limit: Some(Nat::from(self.reserved_cycles)),
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
                        self.initial_cycles as u128
                    )
                }, 3).await
            {
                Ok(canister) => canister.0.canister_id,
                Err(e) => {
                    return Err(NewStorageError::CreateCanisterError(format!("{e:?}")));
                }
            };

            self.sub_canisters.insert(canister_id, Canister {
                canister_id,
                state: CanisterState::Created,
            });
        }

        let init_args = match Encode!(&sub_canister::ArgsStorage::Init(self.init_args.clone())) {
            Ok(encoded_init_args) => encoded_init_args,
            Err(e) => {
                return Err(NewStorageError::FailedToSerializeInitArgs(format!("{e}")));
            }
        };

        // Step 2: Install the Wasm module to the newly created canister
        let install_args = InstallCodeArgument {
            mode: CanisterInstallMode::Install,
            canister_id: canister_id,
            wasm_module: self.wasm.clone(),
            arg: init_args,
        };

        match retry_async(|| install_code(install_args.clone()), 3).await {
            Ok(_) => {}
            Err(e) => {
                return Err(NewStorageError::InstallCodeError(format!("{:?}", e.1)));
            }
        }

        let canister = Canister {
            canister_id,
            state: CanisterState::Installed,
        };

        self.sub_canisters.insert(canister_id, canister.clone());

        Ok(canister)
    }

    async fn update_canisters(&mut self) -> Result<(), Vec<String>> {
        let init_args = match
            Encode!(&sub_canister::ArgsStorage::Upgrade(self.upgrade_args.clone()))
        {
            Ok(encoded_init_args) => encoded_init_args,
            Err(e) => {
                return Err(vec![format!("ERROR : failed to create init args with error - {e}")]);
            }
        };

        let mut canister_upgrade_errors = vec![];

        for (storage_canister_id, canister) in self.sub_canisters.clone().iter() {
            match
                retry_async(|| {
                    stop_canister(CanisterIdRecord {
                        canister_id: *storage_canister_id,
                    })
                }, 3).await
            {
                Ok(_) => {
                    self.sub_canisters.insert(*storage_canister_id, Canister {
                        canister_id: storage_canister_id.clone(),
                        state: CanisterState::Stopped,
                    });
                }
                Err(e) => {
                    canister_upgrade_errors.push(
                        format!(
                            "ERROR: storage upgrade :: storage with principal : {} failed to stop with error {:?}",
                            *storage_canister_id,
                            e
                        )
                    );
                    continue;
                }
            }

            let result = {
                let init_args = init_args.clone();
                let wasm_module = self.wasm.clone();

                let install_args = InstallCodeArgument {
                    mode: CanisterInstallMode::Upgrade(None),
                    canister_id: *storage_canister_id,
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
                                canister_id: *storage_canister_id,
                            })
                        }, 3).await
                    {
                        Ok(_) => {
                            self.sub_canisters.insert(*storage_canister_id, Canister {
                                canister_id: storage_canister_id.clone(),
                                state: CanisterState::Installed,
                            });
                        }
                        Err(e) => {
                            canister_upgrade_errors.push(
                                format!(
                                    "ERROR: storage upgrade :: storage with principal : {} failed to start with error {:?}",
                                    *storage_canister_id,
                                    e
                                )
                            );
                        }
                    }
                }
                Err(e) => {
                    canister_upgrade_errors.push(
                        format!(
                            "ERROR: storage upgrade :: storage with principal : {} failed to install upgrade {:?}",
                            *storage_canister_id,
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

    fn list_canisters(&self) -> Vec<Canister> {
        self.sub_canisters.clone().into_values().collect()
    }

    fn list_canisters_ids(&self) -> Vec<Principal> {
        self.sub_canisters.clone().into_keys().collect()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StorageSubCanisterManager {
    sub_canister_manager: SubCanisterManager<sub_canister::InitArgs, sub_canister::UpgradeArgs>,
}

impl StorageSubCanisterManager {
    pub fn new(
        init_args: sub_canister::InitArgs,
        upgrade_args: sub_canister::UpgradeArgs,
        master_canister_id: Principal,
        sub_canisters: HashMap<Principal, Canister>,
        controllers: Vec<Principal>,
        authorized_principal: Vec<Principal>,
        initial_cycles: u128,
        reserved_cycles: u128,
        test_mode: bool,
        commit_hash: String,
        wasm: Vec<u8>
    ) -> Self {
        Self {
            sub_canister_manager: SubCanisterManager::new(
                init_args,
                upgrade_args,
                master_canister_id,
                sub_canisters,
                controllers,
                authorized_principal,
                initial_cycles,
                reserved_cycles,
                test_mode,
                commit_hash,
                wasm
            ),
        }
    }

    pub async fn insert_data(
        &mut self,
        data: Value,
        data_id: Nat,
        nft_id: Nat
    ) -> Result<(String, Canister), String> {
        let required_space = get_value_size(data.clone());

        for canister in self.sub_canister_manager.sub_canisters.values() {
            match canister.get_storage_size().await {
                Ok(size) if size + required_space <= MAX_STORAGE_SIZE => {
                    match canister.insert_data(data.clone(), data_id.clone(), nft_id.clone()).await {
                        Ok(hash_id) => {
                            return Ok((hash_id, canister.clone()));
                        }
                        Err(_) => {
                            continue;
                        }
                    }
                }
                _ => {
                    continue;
                }
            }
        }

        // if no canister has enough space, create a new one
        match self.sub_canister_manager.create_canister().await {
            Ok(new_canister) => {
                match new_canister.insert_data(data.clone(), data_id.clone(), nft_id.clone()).await {
                    Ok(hash_id) => { Ok((hash_id, new_canister)) }
                    Err(e) => { Err(format!("{e:?}")) }
                }
            }
            Err(e) => { Err(format!("{e:?}")) }
        }
    }

    pub async fn get_data(&self, canister: Canister, hash_id: String) -> Result<Value, String> {
        canister.get_data(hash_id).await
    }
}

impl SubCanister for StorageSubCanisterManager {
    type InitArgs = sub_canister::InitArgs;
    type UpgradeArgs = sub_canister::UpgradeArgs;

    async fn create_canister(&mut self) -> Result<Canister, NewStorageError> {
        self.sub_canister_manager.create_canister().await
    }

    async fn update_canisters(&mut self) -> Result<(), Vec<String>> {
        self.sub_canister_manager.update_canisters().await
    }

    fn list_canisters(&self) -> Vec<Canister> {
        self.sub_canister_manager.list_canisters()
    }

    fn list_canisters_ids(&self) -> Vec<Principal> {
        self.sub_canister_manager.list_canisters_ids()
    }
}

// TODO separate in another file ?
#[derive(Serialize, Deserialize, Clone)]
enum CanisterError {
    CantFindControllers(String),
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum CanisterState {
    Created,
    Installed,
    Stopped,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Canister {
    pub canister_id: Principal,
    state: CanisterState,
}

impl Canister {
    pub async fn get_storage_size(&self) -> Result<u128, String> {
        let res = retry_async(|| get_storage_size(self.canister_id, &()), 3).await;

        trace(&format!("Checking storage : {:?}. storage size {res:?}.", self.canister_id));

        match res {
            Ok(size) => Ok(size),
            Err(err) => Err(err.1),
        }
    }

    async fn get_canister_controllers(&self) -> Result<Vec<Principal>, CanisterError> {
        match
            retry_async(
                || canister_status(CanisterIdRecord { canister_id: self.canister_id }),
                3
            ).await
        {
            Ok(res) => Ok(res.0.settings.controllers),
            Err(e) => Err(CanisterError::CantFindControllers(format!("{e:?}"))),
        }
    }

    async fn insert_data(&self, data: Value, data_id: Nat, nft_id: Nat) -> Result<String, String> {
        if self.state != CanisterState::Installed {
            return Err("Canister is not installed".to_string());
        }

        let res = retry_async(
            ||
                insert_data(self.canister_id, insert_data::Args {
                    data: data.clone(),
                    data_id: data_id.clone(),
                    nft_id: nft_id.clone(),
                }),
            3
        ).await;

        match res {
            Ok(data_response) => Ok(data_response.hash_id),
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub async fn get_data(&self, hash_id: String) -> Result<Value, String> {
        let res = retry_async(
            || get_data(self.canister_id, get_data::Args { hash_id: hash_id.clone() }),
            3
        ).await;

        match res {
            Ok(data) => Ok(data.data_value),
            Err(e) => Err(format!("{e:?}")),
        }
    }
}
