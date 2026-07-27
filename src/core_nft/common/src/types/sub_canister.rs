use crate::types::management::{
    cancel_upload, finalize_upload, init_upload, remove_file, store_chunk,
};
use crate::utils::trace;
use bity_ic_storage_canister_api::init_reupload;
use bity_ic_storage_canister_c2c::get_stored_files_size_bytes;
use bity_ic_storage_canister_c2c::init_reupload;
use bity_ic_storage_canister_c2c::{
    cancel_upload, finalize_upload, get_storage_size, init_upload, remove_file, store_chunk,
};
use bity_ic_subcanister_manager;
use bity_ic_subcanister_manager::Canister;
use bity_ic_utils::retry_async::retry_async;
use candid::{CandidType, Principal};
use canfund::manager::options::{CyclesThreshold, FundManagerOptions, FundStrategy};
use ic_cdk::management_canister::{canister_status, CanisterStatusArgs};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const MAX_STORAGE_SIZE: u128 = 500 * 1024 * 1024 * 1024; // 500 GiB TODO maybe we should put a be less here ?
const MAX_FILE_SIZE: u128 = 2 * 1024 * 1024 * 1024; // 2 GiB

pub const INITIAL_CYCLES_BALANCE: u128 = 5_000_000_000_000; // 5T cycles
pub const RESERVED_CYCLES_BALANCE: u128 = 2_000_000_000_000; // 2T cycles

// A test-mode storage canister burns ~1.3B cycles/day, so 0.5T is over a year of
// runway. It is funded out of the parent collection's balance, and the manager's
// top-up floor has to stay above this plus fees for a spawn to be possible at all.
pub const INITIAL_CYCLES_BALANCE_TEST_MODE: u128 = 500_000_000_000; // 0.5T cycles
// `reserved_cycles_limit` is a ceiling on what the sub-canister may reserve for
// memory allocation, but the IC requires the parent to hold it liquid at spawn
// time on top of the 0.5T creation fee and the initial balance. At 0.5T that made
// a spawn cost ~1.55T, which is what the manager's top-up floor has to clear.
// Staging canisters allocate little, so 0.1T is ample and drops a spawn to ~1.1T.
pub const RESERVED_CYCLES_BALANCE_TEST_MODE: u128 = 100_000_000_000; // 0.1T cycles

pub use bity_ic_storage_canister_api::lifecycle::Args as ArgsStorage;

#[derive(Serialize, Deserialize, Clone)]
pub struct StorageSubCanisterManager {
    pub sub_canister_manager: bity_ic_subcanister_manager::SubCanisterManager<StorageCanister>,
    init_args: ArgsStorage,
    upgrade_args: ArgsStorage,
}

/// Canfund options for storage sub-canisters. `SubCanisterManager.funding_config`
/// is `#[serde(skip)]`, so this must be re-applied after every upgrade, not just
/// at init.
///
/// Test mode gets its own thresholds, scaled to `INITIAL_CYCLES_BALANCE_TEST_MODE`.
/// Under the production settings a test-mode storage canister was created below
/// canfund's 1 TC floor and refilled to 3 TC the instant it existed, out of the
/// parent collection's balance. That drop pushed the collection under the manager's
/// top-up floor, which is how a staging collection ended up costing 7.5 TC. Keeping
/// the floor and refill below the endowment leaves it at what it was given.
///
/// The interval is also relaxed in test mode: every tick is a `canister_status`
/// call per storage child, paid for by the collection, and on staging that dominates
/// the collection's cycle burn.
pub fn default_funding_config(test_mode: bool) -> FundManagerOptions {
    let (interval_secs, min_cycles, fund_cycles) = if test_mode {
        (3600, 300_000_000_000, 500_000_000_000)
    } else {
        (60, 1_000_000_000_000, 2_000_000_000_000)
    };

    FundManagerOptions::new()
        .with_interval_secs(interval_secs)
        .with_strategy(FundStrategy::BelowThreshold(
            CyclesThreshold::new()
                .with_min_cycles(min_cycles)
                .with_fund_cycles(fund_cycles),
        ))
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
        let funding_config = default_funding_config(test_mode);

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
            let storage_size = canister.get_storage_size().await;
            let stored_files_size = canister.get_stored_files_size_bytes().await;
            match (storage_size, stored_files_size) {
                (Ok(size), Ok(files_size)) => {
                    let expected_size = size.max((files_size as u128) + file_size);
                    if expected_size <= MAX_STORAGE_SIZE {
                        match canister.init_upload(data.clone()).await {
                            Ok(_) => {
                                trace(&format!("Initialized upload"));
                                return Ok((
                                    init_upload::InitUploadResp {},
                                    canister.canister_id(),
                                ));
                            }
                            Err(_) => {
                                continue;
                            }
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
        let res = retry_async(|| get_storage_size(self.canister_id, ()), 3).await;

        trace(&format!(
            "Checking storage : {:?}. storage size {res:?}.",
            self.canister_id
        ));

        match res {
            Ok(size) => Ok(size),
            Err(err) => Err(err),
        }
    }

    pub async fn get_stored_files_size_bytes(&self) -> Result<u64, String> {
        let res = retry_async(|| get_stored_files_size_bytes(self.canister_id, ()), 3).await;

        trace(&format!(
            "Checking storage : {:?}. storage size {res:?}.",
            self.canister_id
        ));

        match res {
            Ok(size) => Ok(size),
            Err(err) => Err(err),
        }
    }

    #[allow(dead_code)]
    async fn get_canister_controllers(&self) -> Result<Vec<Principal>, CanisterError> {
        let canister_status_args = CanisterStatusArgs {
            canister_id: self.canister_id,
        };

        match retry_async(|| canister_status(&canister_status_args), 3).await {
            Ok(res) => Ok(res.settings.controllers),
            Err(e) => Err(CanisterError::CantFindControllers(format!("{e:?}"))),
        }
    }

    pub async fn init_upload(
        &self,
        data: init_upload::Args,
    ) -> crate::types::management::init_upload::Response {
        if self.state != bity_ic_subcanister_manager::CanisterState::Installed {
            return Err(
                crate::types::management::init_upload::InitUploadError::StorageCanisterError(
                    "Canister is not installed".to_string(),
                ),
            );
        }

        let res = retry_async(|| init_upload(self.canister_id, data.clone()), 3).await;
        trace(&format!("init_upload response: {:?}", res));

        match res {
            Ok(init_upload_response) => {
                crate::types::management::init_upload::from_storage_response(init_upload_response)
            }
            Err(e) => Err(
                crate::types::management::init_upload::InitUploadError::StorageCanisterError(
                    format!("{e:?}"),
                ),
            ),
        }
    }

    pub async fn init_reupload(
        &self,
        data: init_reupload::Args,
    ) -> crate::types::management::init_reupload::Response {
        if self.state != bity_ic_subcanister_manager::CanisterState::Installed {
            return Err(
                crate::types::management::init_reupload::InitReuploadError::StorageCanisterError(
                    "Canister is not installed".to_string(),
                ),
            );
        }

        let res = retry_async(|| init_reupload(self.canister_id, data.clone()), 3).await;
        trace(&format!("init_reupload response: {:?}", res));

        match res {
            Ok(init_reupload_response) => {
                crate::types::management::init_reupload::from_storage_response(
                    init_reupload_response,
                )
            }
            Err(e) => Err(
                crate::types::management::init_reupload::InitReuploadError::StorageCanisterError(
                    format!("{e:?}"),
                ),
            ),
        }
    }

    pub async fn store_chunk(
        &self,
        data: store_chunk::Args,
    ) -> crate::types::management::store_chunk::Response {
        if self.state != bity_ic_subcanister_manager::CanisterState::Installed {
            return Err(
                crate::types::management::store_chunk::StoreChunkError::StorageCanisterError(
                    "Canister is not installed".to_string(),
                ),
            );
        }

        let res = retry_async(|| store_chunk(self.canister_id, data.clone()), 3).await;
        trace(&format!("store_chunk response: {:?}", res));

        match res {
            Ok(store_chunk_response) => {
                crate::types::management::store_chunk::from_storage_response(store_chunk_response)
            }
            Err(e) => Err(
                crate::types::management::store_chunk::StoreChunkError::StorageCanisterError(
                    format!("{e:?}"),
                ),
            ),
        }
    }

    pub async fn finalize_upload(
        &self,
        data: finalize_upload::Args,
    ) -> crate::types::management::finalize_upload::Response {
        if self.state != bity_ic_subcanister_manager::CanisterState::Installed {
            return Err(
                crate::types::management::finalize_upload::FinalizeUploadError::StorageCanisterError(
                    "Canister is not installed".to_string(),
                ),
            );
        }

        let res = retry_async(|| finalize_upload(self.canister_id, data.clone()), 3).await;

        match res {
            Ok(finalize_upload_response) => {
                crate::types::management::finalize_upload::from_storage_response(
                    finalize_upload_response,
                )
            }
            Err(e) => Err(
                crate::types::management::finalize_upload::FinalizeUploadError::StorageCanisterError(
                    format!("{e:?}"),
                ),
            ),
        }
    }

    pub async fn cancel_upload(
        &self,
        data: cancel_upload::Args,
    ) -> crate::types::management::cancel_upload::Response {
        if self.state != bity_ic_subcanister_manager::CanisterState::Installed {
            return Err(
                crate::types::management::cancel_upload::CancelUploadError::StorageCanisterError(
                    "Canister is not installed".to_string(),
                ),
            );
        }

        let res = retry_async(|| cancel_upload(self.canister_id, data.clone()), 3).await;

        match res {
            Ok(cancel_upload_response) => {
                crate::types::management::cancel_upload::from_storage_response(
                    cancel_upload_response,
                )
            }
            Err(e) => Err(
                crate::types::management::cancel_upload::CancelUploadError::StorageCanisterError(
                    format!("{e:?}"),
                ),
            ),
        }
    }

    pub async fn remove_file(
        &self,
        data: remove_file::Args,
    ) -> crate::types::management::remove_file::Response {
        if self.state != bity_ic_subcanister_manager::CanisterState::Installed {
            return Err(
                crate::types::management::remove_file::RemoveFileError::StorageCanisterError(
                    "Canister is not installed".to_string(),
                ),
            );
        }

        let res = retry_async(|| remove_file(self.canister_id, data.clone()), 3).await;

        match res {
            Ok(resp) => crate::types::management::remove_file::from_storage_response(resp),
            Err(e) => Err(
                crate::types::management::remove_file::RemoveFileError::StorageCanisterError(
                    format!("{e:?}"),
                ),
            ),
        }
    }

    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
