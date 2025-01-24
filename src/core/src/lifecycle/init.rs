use crate::lifecycle::init_canister;
use crate::lifecycle::Args;
use crate::state::{ Data, RuntimeState };
use candid::{ CandidType, Nat };
use canister_tracing_macros::trace;
use ic_cdk_macros::init;
use serde::{ Deserialize, Serialize };
use tracing::info;
use utils::env::{ CanisterEnv, Environment };
use candid::Principal;
use crate::state::InitApprovalsArg;
use types::BuildVersion;
use crate::types::collection_metadata::CollectionMetadata;

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct InitArgs {
    test_mode: bool,
    version: BuildVersion,
    commit_hash: String,
    authorized_principals: Vec<Principal>,
    description: Option<String>,
    symbol: String,
    name: String,
    logo: Option<Vec<u8>>,
    supply_cap: Option<Nat>,
    max_query_batch_size: Option<Nat>,
    max_update_batch_size: Option<Nat>,
    max_take_value: Option<Nat>,
    default_take_value: Option<Nat>,
    max_memo_size: Option<Nat>,
    atomic_batch_transfers: Option<bool>,
    tx_window: Option<Nat>,
    permitted_drift: Option<Nat>,
    collection_metadata: CollectionMetadata,
    approval_init: Option<InitApprovalsArg>,
}

#[init]
#[trace]
fn init(args: Args) {
    match args {
        Args::Init(init_args) => {
            let env = CanisterEnv::new(
                init_args.test_mode,
                init_args.version,
                init_args.commit_hash
            );
            let mut data = Data::new(
                init_args.authorized_principals,
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
                init_args.collection_metadata,
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
