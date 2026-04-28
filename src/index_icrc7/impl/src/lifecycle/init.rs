use crate::lifecycle::{init_canister, Args};
use crate::state::{Data, RuntimeState};
use bity_ic_canister_tracing_macros::trace;
use bity_ic_utils::env::{CanisterEnv, Environment};
use ic_cdk_macros::init;
use tracing::info;

#[init]
#[trace]
fn init(args: Args) {
    match args {
        Args::Init(init_args) => {
            bity_ic_canister_logger::init(init_args.test_mode);

            let env = CanisterEnv::new(
                init_args.test_mode,
                init_args.version,
                init_args.commit_hash.clone(),
            );

            // let mut data = Data::new(init_args.authorized_principals);
            let mut data = Data::new(
                init_args.authorized_principals,
                init_args.ledger_canister_id,
            );

            if init_args.test_mode {
                data.authorized_principals.insert(env.caller());
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
