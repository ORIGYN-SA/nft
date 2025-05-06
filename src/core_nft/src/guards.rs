use crate::state::mutate_state;
use crate::state::read_state;
use candid::Principal;
use std::marker::PhantomData;

const MAX_CONCURRENT: usize = 1;

/// Guards a block from executing twice when called by the same user and from being
/// executed [MAX_CONCURRENT] or more times in parallel.
#[must_use]
pub struct GuardManagement {
    principal: Principal,
    _marker: PhantomData<GuardManagement>,
}

impl GuardManagement {
    /// Attempts to create a new guard for the current block. Fails if there is
    /// already a pending request for the specified [principal] or if there
    /// are at least [MAX_CONCURRENT] pending requests.
    pub fn new(principal: Principal) -> Result<Self, String> {
        mutate_state(|s| {
            if s.principal_guards.len() >= MAX_CONCURRENT {
                return Err(
                    "Service is already running a management query, try again shortly".into(),
                );
            }
            s.principal_guards.insert(principal);
            Ok(Self {
                principal,
                _marker: PhantomData,
            })
        })
    }
}

impl Drop for GuardManagement {
    fn drop(&mut self) {
        mutate_state(|s| s.principal_guards.remove(&self.principal));
    }
}

pub fn caller_is_governance_principal() -> Result<(), String> {
    if read_state(|state| state.is_caller_governance_principal()) {
        Ok(())
    } else {
        Err("Caller is not a governance principal".to_string())
    }
}

pub fn caller_is_minting_authority() -> Result<(), String> {
    if read_state(|state| state.is_caller_minting_authority()) {
        Ok(())
    } else {
        Err("Caller is not a minting authority".to_string())
    }
}
