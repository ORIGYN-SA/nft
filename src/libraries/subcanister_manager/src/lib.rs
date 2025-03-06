use std::collections::HashMap;
use std::future::Future;

use candid::{ CandidType, Encode, Nat, Principal };
use ic_cdk::api::management_canister::main::{
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
use utils::retry_async::retry_async;
use std::fmt::Debug;
use std::any::Any;

#[derive(Debug)]
pub enum NewCanisterError {
    CreateCanisterError(String),
    InstallCodeError(String),
    FailedToSerializeInitArgs(String),
}

pub trait SubCanister {
    type Canister: Send + Sync;

    fn create_canister(
        &mut self
    ) -> impl Future<Output = Result<Box<impl Canister>, NewCanisterError>> + Send + '_; // Explicitly capture lifetime
    fn update_canisters(&mut self) -> impl Future<Output = Result<(), Vec<String>>> + Send + '_; // Explicitly capture lifetime
    fn list_canisters(&self) -> Vec<Box<impl Canister>>;
    fn list_canisters_ids(&self) -> Vec<Principal>;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SubCanisterManager<T> where T: Canister + Clone + Serialize + Debug + Send + Sync {
    pub master_canister_id: Principal,
    pub sub_canisters: HashMap<Principal, Box<T>>,
    pub controllers: Vec<Principal>,
    pub authorized_principal: Vec<Principal>,
    pub initial_cycles: u128,
    pub reserved_cycles: u128,
    pub test_mode: bool,
    pub commit_hash: String,
    pub wasm: Vec<u8>,
}

impl<T> SubCanisterManager<T> where T: Canister + Clone + Serialize + Debug + Send + Sync {
    pub fn new(
        master_canister_id: Principal,
        sub_canisters: HashMap<Principal, Box<T>>,
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

    pub fn create_canister<U>(
        &mut self,
        init_args: U
    ) -> impl Future<Output = Result<Box<impl Canister + Debug + Clone>, NewCanisterError>> +
            Send +
            '_
        where U: Clone + Serialize + CandidType + Send + Sync + 'static
    {
        async move {
            // find in self.sub_canisters if a canister is already created but not installed
            let mut canister_id = Principal::anonymous();

            for (_canister_id, canister) in self.sub_canisters.iter() {
                if canister.state() == CanisterState::Created {
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
                        return Err(NewCanisterError::CreateCanisterError(format!("{e:?}")));
                    }
                };

                self.sub_canisters.insert(
                    canister_id,
                    Box::new(T::new(canister_id, CanisterState::Created))
                );
            }

            let init_args = match Encode!(&init_args.clone()) {
                Ok(encoded_init_args) => encoded_init_args,
                Err(e) => {
                    return Err(NewCanisterError::FailedToSerializeInitArgs(format!("{e}")));
                }
            };

            // Step 2: Install the Wasm module to the newly created canister
            let install_args = InstallCodeArgument {
                mode: CanisterInstallMode::Install,
                canister_id: canister_id,
                wasm_module: self.wasm.clone(),
                arg: init_args,
            };

            match install_code(install_args.clone()).await {
                Ok(_) => {}
                Err(e) => {
                    return Err(NewCanisterError::InstallCodeError(format!("{:?}", e.1)));
                }
            }
            let canister = Box::new(T::new(canister_id, CanisterState::Installed));

            self.sub_canisters.insert(canister_id, canister.clone());

            Ok(canister)
        }
    }

    pub fn update_canisters<U>(
        &mut self,
        update_args: U
    ) -> impl Future<Output = Result<(), Vec<String>>> + Send + '_
        where U: Clone + Serialize + CandidType + Send + Sync + 'static
    {
        async move {
            let init_args = match Encode!(&update_args.clone()) {
                Ok(encoded_init_args) => encoded_init_args,
                Err(e) => {
                    return Err(
                        vec![format!("ERROR : failed to create init args with error - {e}")]
                    );
                }
            };

            let mut canister_upgrade_errors = vec![];

            for (canister_id, _) in self.sub_canisters.clone().iter() {
                match
                    retry_async(|| {
                        stop_canister(CanisterIdRecord {
                            canister_id: *canister_id,
                        })
                    }, 3).await
                {
                    Ok(_) => {
                        self.sub_canisters.insert(
                            *canister_id,
                            Box::new(T::new(*canister_id, CanisterState::Stopped))
                        );
                    }
                    Err(e) => {
                        canister_upgrade_errors.push(
                            format!(
                                "ERROR: storage upgrade :: storage with principal : {} failed to stop with error {:?}",
                                *canister_id,
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
                        canister_id: *canister_id,
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
                                    canister_id: *canister_id,
                                })
                            }, 3).await
                        {
                            Ok(_) => {
                                self.sub_canisters.insert(
                                    *canister_id,
                                    Box::new(T::new(*canister_id, CanisterState::Installed))
                                );
                            }
                            Err(e) => {
                                canister_upgrade_errors.push(
                                    format!(
                                        "ERROR: storage upgrade :: storage with principal : {} failed to start with error {:?}",
                                        *canister_id,
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
                                *canister_id,
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
    }

    pub fn list_canisters(&self) -> Vec<Box<impl Canister>> {
        self.sub_canisters.values().cloned().collect()
    }

    pub fn list_canisters_ids(&self) -> Vec<Principal> {
        self.sub_canisters.clone().into_keys().collect()
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum CanisterState {
    Created,
    Installed,
    Stopped,
}

pub trait Canister {
    fn new(canister_id: Principal, state: CanisterState) -> Self;
    fn canister_id(&self) -> Principal;
    fn state(&self) -> CanisterState;
    fn as_any(&self) -> &dyn Any;
}
