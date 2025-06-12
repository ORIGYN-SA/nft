use std::collections::HashMap;

use crate::utils::trace;
use bity_ic_subcanister_manager;
use bity_ic_subcanister_manager::Canister;
use bity_ic_utils::retry_async::retry_async;
use candid::{CandidType, Principal};
use canfund::manager::options::{CyclesThreshold, FundManagerOptions, FundStrategy};
use ic_cdk::api::management_canister::main::{canister_status, CanisterIdRecord};
use serde::{Deserialize, Serialize};
use storage_api_canister::cancel_upload;
use storage_api_canister::finalize_upload;
use storage_api_canister::init_upload;
use storage_api_canister::store_chunk;
use storage_canister_c2c_client::{
    cancel_upload, finalize_upload, get_storage_size, init_upload, store_chunk,
};

const MAX_STORAGE_SIZE: u128 = 500 * 1024 * 1024 * 1024; // 500 GiB TODO maybe we should put a be less here ?
const MAX_FILE_SIZE: u128 = 2 * 1024 * 1024 * 1024; // 2 GiB

pub use storage_api_canister::lifecycle::Args as ArgsStorage;

#[derive(Serialize, Deserialize, Clone)]
pub struct StorageSubCanisterManager {
    sub_canister_manager: bity_ic_subcanister_manager::SubCanisterManager<StorageCanister>,
    init_args: ArgsStorage,
    upgrade_args: ArgsStorage,
}

impl StorageSubCanisterManager {
    pub fn new(
        init_args: ArgsStorage,
        upgrade_args: ArgsStorage,
        master_canister_id: Principal,
        sub_canisters: HashMap<Principal, Box<StorageCanister>>,
        controllers: Vec<Principal>,
        authorized_principal: Vec<Principal>,
        initial_cycles: u128,
        reserved_cycles: u128,
        test_mode: bool,
        commit_hash: String,
        wasm: Vec<u8>,
    ) -> Self {
        let funding_config = FundManagerOptions::new()
            .with_interval_secs(60)
            .with_strategy(FundStrategy::BelowThreshold(
                CyclesThreshold::new()
                    .with_min_cycles(1_000_000_000_000)
                    .with_fund_cycles(2_000_000_000_000),
            ));

        Self {
            sub_canister_manager: bity_ic_subcanister_manager::SubCanisterManager::new(
                master_canister_id,
                sub_canisters,
                controllers,
                authorized_principal,
                initial_cycles,
                reserved_cycles,
                test_mode,
                commit_hash,
                wasm,
                funding_config,
            ),
            init_args,
            upgrade_args,
        }
    }

    pub async fn init_upload(
        &mut self,
        data: init_upload::Args,
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
                            return Ok((init_upload::InitUploadResp {}, canister.canister_id()));
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
        match self
            .sub_canister_manager
            .create_canister(self.init_args.clone())
            .await
        {
            Ok(new_canister) => {
                trace(&format!(
                    "Created a new canister with principal: {:?}",
                    new_canister
                ));
                if let Some(storage_canister) =
                    (*new_canister).as_any().downcast_ref::<StorageCanister>()
                {
                    match storage_canister.init_upload(data.clone()).await {
                        Ok(_) => {
                            trace(&format!("Initialized upload"));

                            Ok((
                                init_upload::InitUploadResp {},
                                storage_canister.canister_id(),
                            ))
                        }
                        Err(e) => Err(format!("{e:?}")),
                    }
                } else {
                    Err("Failed to cast to StorageCanister".to_string())
                }
            }
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub fn get_canister(&self, canister_id: Principal) -> Option<StorageCanister> {
        match self.sub_canister_manager.sub_canisters.get(&canister_id) {
            Some(canister) => Some(*canister.clone()),
            None => None,
        }
    }

    fn get_subcanisters_installed(&self) -> Vec<StorageCanister> {
        self.sub_canister_manager
            .list_canisters()
            .into_iter()
            .filter_map(|canister| {
                if canister.state() == bity_ic_subcanister_manager::CanisterState::Installed {
                    canister.as_any().downcast_ref::<StorageCanister>().cloned()
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn list_canisters(&self) -> Vec<Box<impl Canister>> {
        self.sub_canister_manager.list_canisters()
    }

    pub fn list_canisters_ids(&self) -> Vec<Principal> {
        self.sub_canister_manager.list_canisters_ids()
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct StorageCanister {
    canister_id: Principal,
    state: bity_ic_subcanister_manager::CanisterState,
    canister_param: ArgsStorage,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum CanisterError {
    CantFindControllers(String),
}

impl bity_ic_subcanister_manager::Canister for StorageCanister {
    type ParamType = ArgsStorage;

    fn new(
        canister_id: Principal,
        state: bity_ic_subcanister_manager::CanisterState,
        canister_param: Self::ParamType,
    ) -> Self {
        Self {
            canister_id,
            state,
            canister_param,
        }
    }

    fn canister_id(&self) -> Principal {
        self.canister_id.clone()
    }

    fn state(&self) -> bity_ic_subcanister_manager::CanisterState {
        self.state.clone()
    }

    fn canister_param(&self) -> Self::ParamType {
        self.canister_param.clone()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl StorageCanister {
    pub async fn get_storage_size(&self) -> Result<u128, String> {
        let res = retry_async(|| get_storage_size(self.canister_id, &()), 3).await;

        trace(&format!(
            "Checking storage : {:?}. storage size {res:?}.",
            self.canister_id
        ));

        match res {
            Ok(size) => Ok(size),
            Err(err) => Err(err.1),
        }
    }

    #[allow(dead_code)]
    async fn get_canister_controllers(&self) -> Result<Vec<Principal>, CanisterError> {
        match retry_async(
            || {
                canister_status(CanisterIdRecord {
                    canister_id: self.canister_id,
                })
            },
            3,
        )
        .await
        {
            Ok(res) => Ok(res.0.settings.controllers),
            Err(e) => Err(CanisterError::CantFindControllers(format!("{e:?}"))),
        }
    }

    pub async fn init_upload(
        &self,
        data: init_upload::Args,
    ) -> Result<init_upload::InitUploadResp, String> {
        if self.state != bity_ic_subcanister_manager::CanisterState::Installed {
            return Err("Canister is not installed".to_string());
        }

        let res = retry_async(|| init_upload(self.canister_id, data.clone()), 3).await;

        match res {
            Ok(init_upload_response) => match init_upload_response {
                Ok(data) => Ok(data),
                Err(e) => Err(format!("{e:?}")),
            },
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub async fn store_chunk(
        &self,
        data: store_chunk::Args,
    ) -> Result<store_chunk::StoreChunkResp, String> {
        if self.state != bity_ic_subcanister_manager::CanisterState::Installed {
            return Err("Canister is not installed".to_string());
        }

        let res = retry_async(|| store_chunk(self.canister_id, data.clone()), 3).await;

        match res {
            Ok(store_chunk_response) => match store_chunk_response {
                Ok(data) => Ok(data),
                Err(e) => Err(format!("{e:?}")),
            },
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub async fn finalize_upload(
        &self,
        data: finalize_upload::Args,
    ) -> Result<finalize_upload::FinalizeUploadResp, String> {
        if self.state != bity_ic_subcanister_manager::CanisterState::Installed {
            return Err("Canister is not installed".to_string());
        }

        let res = retry_async(|| finalize_upload(self.canister_id, data.clone()), 3).await;

        match res {
            Ok(finalize_upload_response) => match finalize_upload_response {
                Ok(data) => Ok(data),
                Err(e) => Err(format!("{e:?}")),
            },
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub async fn cancel_upload(
        &self,
        data: cancel_upload::Args,
    ) -> Result<cancel_upload::CancelUploadResp, String> {
        if self.state != bity_ic_subcanister_manager::CanisterState::Installed {
            return Err("Canister is not installed".to_string());
        }

        let res = retry_async(|| cancel_upload(self.canister_id, data.clone()), 3).await;

        match res {
            Ok(cancel_upload_response) => match cancel_upload_response {
                Ok(data) => Ok(data),
                Err(e) => Err(format!("{e:?}")),
            },
            Err(e) => Err(format!("{e:?}")),
        }
    }

    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
