use ic_cdk::api::call::CallResult;
use ic_cdk::api::management_canister;
use ic_cdk::api::management_canister::main::CanisterIdRecord;
use tracing::{error, info};
use types::{CanisterId, Cycles};

pub async fn deposit_cycles(canister_id: CanisterId, amount: Cycles) -> CallResult<()> {
    if let Err((code, msg)) =
        management_canister::main::deposit_cycles(CanisterIdRecord { canister_id }, amount.into())
            .await
    {
        error!(
            %canister_id,
            error_code = code as u8,
            error_message = msg.as_str(),
            "Error calling 'deposit_cycles'"
        );
        Err((code, msg))
    } else {
        info!(
            %canister_id,
            amount,
            "Topped up canister with cycles"
        );
        Ok(())
    }
}

pub async fn get_cycles_balance(canister_id: CanisterId) -> Result<u64, String> {
    match ic_cdk::api::management_canister::main::canister_status(CanisterIdRecord { canister_id })
        .await
    {
        Ok(res) => {
            let as_u64: Result<u64, _> = res.0.cycles.0.try_into();
            match as_u64 {
                Ok(amount) => Ok(amount),
                Err(e) => Err(format!("{e:?}")),
            }
        }
        Err(e) => Err(format!("{e:?}")),
    }
}
