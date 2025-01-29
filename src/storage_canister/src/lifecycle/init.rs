use crate::lifecycle::init_canister;
use crate::lifecycle::Args;
use crate::state::{ Data, RuntimeState };
use candid::CandidType;
use canister_tracing_macros::trace;
use ic_cdk_macros::init;
use serde::{ Deserialize, Serialize };
use tracing::info;
use utils::env::{ CanisterEnv, Environment };
use candid::Principal;
use types::BuildVersion;

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct InitArgs {
    test_mode: bool,
    version: BuildVersion,
    commit_hash: String,
    authorized_principals: Vec<Principal>,
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
            let mut data = Data::new(init_args.authorized_principals);

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
