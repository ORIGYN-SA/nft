use std::collections::HashMap;

use crate::types::sub_canister;
use crate::utils::trace;
use candid::{ CandidType, Encode, Nat, Principal };
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
use serde::{ Deserialize, Serialize };
use storage_api_canister::cancel_upload;
use storage_api_canister::delete_file;
use storage_api_canister::types::value_custom::CustomValue as Value;
use storage_api_canister::utils::get_value_size;
use storage_canister_c2c_client::{
    get_data,
    get_storage_size,
    insert_data,
    init_upload,
    store_chunk,
    finalize_upload,
    cancel_upload,
    delete_file,
};
use utils::retry_async::retry_async;
use storage_api_canister::init_upload;
use storage_api_canister::store_chunk;
use storage_api_canister::finalize_upload;

#[derive(Debug)]
pub enum NewStorageError {
    CreateCanisterError(String),
    InstallCodeError(String),
    FailedToSerializeInitArgs(String),
}

const MAX_STORAGE_SIZE: u128 = 500 * 1024 * 1024 * 1024; // 500 GiB TODO maybe we should put a be less here ?
const MAX_FILE_SIZE: u128 = 2 * 1024 * 1024 * 1024; // 2 GiB

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
            canister_id = match
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
                trace(&format!("ERROR : failed to install code with error - {e:?}"));
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
        data_id: String,
        nft_id: Option<Nat>
    ) -> Result<(String, Canister), String> {
        let required_space = get_value_size(data.clone());
        trace(&format!("SubCanisterManager Insert Data : {:?}", data_id));
        trace(&format!("SubCanisterManager required space: {:?}", required_space));

        for canister in self.get_subcanisters_installed() {
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

        trace(&format!("SubCanisterManager no canister available found, create a new one"));
        // if no canister has enough space, create a new one
        match self.sub_canister_manager.create_canister().await {
            Ok(new_canister) => {
                trace(
                    &format!(
                        "SubCanisterManager created a new canister with principal : {:?}",
                        new_canister
                    )
                );

                match new_canister.insert_data(data.clone(), data_id.clone(), nft_id.clone()).await {
                    Ok(hash_id) => {
                        trace(
                            &format!(
                                "SubCanisterManager inserted data with hash_id : {:?}",
                                hash_id
                            )
                        );
                        Ok((hash_id, new_canister))
                    }
                    Err(e) => Err(format!("{e:?}")),
                }
            }
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub async fn init_upload(
        &mut self,
        data: init_upload::Args
    ) -> Result<(init_upload::InitUploadResp, Principal), String> {
        let file_size: u128 = data.file_size as u128;
        if file_size > MAX_FILE_SIZE {
            return Err("File size exceeds the maximum limit of 2GB".to_string());
        }

        for canister in self.get_subcanisters_installed() {
            match canister.get_storage_size().await {
                Ok(size) if size + file_size <= MAX_STORAGE_SIZE => {
                    match canister.init_upload(data.clone()).await {
                        Ok(_) => {
                            trace(&format!("Initialized upload"));
                            return Ok((init_upload::InitUploadResp {}, canister.canister_id));
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

        trace(&format!("No available canister found, creating a new one"));
        match self.sub_canister_manager.create_canister().await {
            Ok(new_canister) => {
                trace(&format!("Created a new canister with principal: {:?}", new_canister));
                match new_canister.init_upload(data.clone()).await {
                    Ok(_) => {
                        trace(&format!("Initialized upload"));
                        Ok((init_upload::InitUploadResp {}, new_canister.canister_id))
                    }
                    Err(e) => Err(format!("{e:?}")),
                }
            }
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub fn get_canister(&self, canister_id: Principal) -> Option<Canister> {
        self.sub_canister_manager.sub_canisters.get(&canister_id).cloned()
    }

    fn get_subcanisters_installed(&self) -> Vec<Canister> {
        self.sub_canister_manager
            .list_canisters()
            .into_iter()
            .filter(|canister| canister.state == CanisterState::Installed)
            .collect()
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

#[derive(CandidType, Serialize, Deserialize, Clone, PartialEq, Debug)]
enum CanisterState {
    Created,
    Installed,
    Stopped,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
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
            retry_async(|| {
                canister_status(CanisterIdRecord {
                    canister_id: self.canister_id,
                })
            }, 3).await
        {
            Ok(res) => Ok(res.0.settings.controllers),
            Err(e) => Err(CanisterError::CantFindControllers(format!("{e:?}"))),
        }
    }

    async fn insert_data(
        &self,
        data: Value,
        data_id: String,
        nft_id: Option<Nat>
    ) -> Result<String, String> {
        if self.state != CanisterState::Installed {
            return Err("Canister is not installed".to_string());
        }

        let res = retry_async(|| {
            insert_data(self.canister_id, insert_data::Args {
                data: data.clone(),
                data_id: data_id.clone(),
                nft_id: nft_id.clone(),
            })
        }, 3).await;

        match res {
            Ok(data_response) => {
                match data_response {
                    Ok(data) => Ok(data.hash_id),
                    Err(e) => Err(format!("{e:?}")),
                }
            }
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub async fn get_data(&self, hash_id: String) -> Result<Value, String> {
        let res = retry_async(|| {
            get_data(self.canister_id, get_data::Args {
                hash_id: hash_id.clone(),
            })
        }, 3).await;

        match res {
            Ok(_data) => {
                match _data {
                    Ok(data) => Ok(data.data_value),
                    Err(e) => Err(format!("{e:?}")),
                }
            }
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub async fn init_upload(
        &self,
        data: init_upload::Args
    ) -> Result<init_upload::InitUploadResp, String> {
        if self.state != CanisterState::Installed {
            return Err("Canister is not installed".to_string());
        }

        let res = retry_async(|| { init_upload(self.canister_id, data.clone()) }, 3).await;

        match res {
            Ok(init_upload_response) => {
                match init_upload_response {
                    Ok(data) => Ok(data),
                    Err(e) => Err(format!("{e:?}")),
                }
            }
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub async fn store_chunk(
        &self,
        data: store_chunk::Args
    ) -> Result<store_chunk::StoreChunkResp, String> {
        if self.state != CanisterState::Installed {
            return Err("Canister is not installed".to_string());
        }

        let res = retry_async(|| { store_chunk(self.canister_id, data.clone()) }, 3).await;

        match res {
            Ok(store_chunk_response) => {
                match store_chunk_response {
                    Ok(data) => Ok(data),
                    Err(e) => Err(format!("{e:?}")),
                }
            }
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub async fn finalize_upload(
        &self,
        data: finalize_upload::Args
    ) -> Result<finalize_upload::FinalizeUploadResp, String> {
        if self.state != CanisterState::Installed {
            return Err("Canister is not installed".to_string());
        }

        let res = retry_async(|| { finalize_upload(self.canister_id, data.clone()) }, 3).await;

        match res {
            Ok(finalize_upload_response) => {
                match finalize_upload_response {
                    Ok(data) => Ok(data),
                    Err(e) => Err(format!("{e:?}")),
                }
            }
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub async fn cancel_upload(
        &self,
        data: cancel_upload::Args
    ) -> Result<cancel_upload::CancelUploadResp, String> {
        if self.state != CanisterState::Installed {
            return Err("Canister is not installed".to_string());
        }

        let res = retry_async(|| { cancel_upload(self.canister_id, data.clone()) }, 3).await;

        match res {
            Ok(cancel_upload_response) => {
                match cancel_upload_response {
                    Ok(data) => Ok(data),
                    Err(e) => Err(format!("{e:?}")),
                }
            }
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub async fn delete_file(
        &self,
        data: delete_file::Args
    ) -> Result<delete_file::DeleteFileResp, String> {
        if self.state != CanisterState::Installed {
            return Err("Canister is not installed".to_string());
        }

        let res = retry_async(|| { delete_file(self.canister_id, data.clone()) }, 3).await;

        match res {
            Ok(delete_file_response) => {
                match delete_file_response {
                    Ok(data) => Ok(data),
                    Err(e) => Err(format!("{e:?}")),
                }
            }
            Err(e) => Err(format!("{e:?}")),
        }
    }
}
