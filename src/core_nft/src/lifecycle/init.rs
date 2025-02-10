use std::collections::HashMap;

use crate::lifecycle::init_canister;
use crate::lifecycle::Args;
use crate::state::InitApprovalsArg;
use crate::state::{ Data, RuntimeState };
use crate::types::collection_metadata;
use crate::types::collection_metadata::CollectionMetadata;
use candid::Principal;
use candid::{ CandidType, Nat };
use canister_tracing_macros::trace;
use ic_cdk_macros::init;
use serde::{ Deserialize, Serialize };
use storage_api_canister::value_custom::CustomValue as Value;
use tracing::info;
use types::BuildVersion;
use utils::env::{ CanisterEnv, Environment };

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct InitArgs {
    pub test_mode: bool,
    pub version: BuildVersion,
    pub commit_hash: String,
    pub authorized_principals: Vec<Principal>,
    pub minting_authorities: Vec<Principal>,
    pub description: Option<String>,
    pub symbol: String,
    pub name: String,
    pub logo: Option<Vec<u8>>,
    pub supply_cap: Option<Nat>,
    pub max_query_batch_size: Option<Nat>,
    pub max_update_batch_size: Option<Nat>,
    pub max_take_value: Option<Nat>,
    pub default_take_value: Option<Nat>,
    pub max_memo_size: Option<Nat>,
    pub atomic_batch_transfers: Option<bool>,
    pub tx_window: Option<Nat>,
    pub permitted_drift: Option<Nat>,
    pub max_canister_storage_threshold: Option<Nat>,
    pub collection_metadata: HashMap<String, Value>,
    pub approval_init: Option<InitApprovalsArg>,
}

#[init]
#[trace]
fn init(args: Args) {
    match args {
        Args::Init(init_args) => {
            info!("Init start.");
            let env = CanisterEnv::new(
                init_args.test_mode,
                init_args.version,
                init_args.commit_hash.clone()
            );
            let collection_metadata: CollectionMetadata = CollectionMetadata::from(
                init_args.collection_metadata
            );
            // let collection_metadata = CollectionMetadata::new();

            let mut data = Data::new(
                init_args.test_mode,
                init_args.commit_hash,
                init_args.version,
                init_args.authorized_principals,
                init_args.minting_authorities,
                init_args.description,
                init_args.symbol,
                init_args.name,
                init_args.logo,
                init_args.supply_cap,
                init_args.max_query_batch_size,
                init_args.max_update_batch_size,
                init_args.max_take_value,
                init_args.default_take_value,
                init_args.max_memo_size,
                init_args.atomic_batch_transfers,
                init_args.tx_window,
                init_args.permitted_drift,
                init_args.max_canister_storage_threshold,
                collection_metadata,
                init_args.approval_init
            );

            if init_args.test_mode {
                data.authorized_principals.push(env.caller());
            }

            let runtime_state = RuntimeState::new(env, data);

            init_canister(runtime_state);

            info!("Init complete.")
        }
        Args::Upgrade(_) => {
            panic!(
                "Cannot initialize the canister with an Upgrade argument. Please provide an Init argument."
            );
        }
    }
}
