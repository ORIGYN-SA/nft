use crate::lifecycle::init_canister;
use storage_api_canister::lifecycle::Args;
use crate::state::{ Data, RuntimeState };
use canister_tracing_macros::trace;
use ic_cdk_macros::init;
use tracing::info;
use utils::env::{ CanisterEnv, Environment };

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
