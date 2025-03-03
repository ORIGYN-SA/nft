use crate::state::read_state;

pub fn caller_is_governance_principal() -> Result<(), String> {
    if read_state(|state| state.is_caller_governance_principal()) {
        Ok(())
    } else {
        Err("Caller is not a governance principal".to_string())
    }
}

pub fn caller_is_self() -> Result<(), String> {
    if ic_cdk::api::id() == ic_cdk::api::caller() {
        Ok(())
    } else {
        Err("Caller is not the canister".to_string())
    }
}
