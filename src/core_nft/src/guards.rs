use crate::state::mutate_state;
use crate::state::read_state;
use crate::types::permissions::Permission;
use bity_ic_types::TimestampNanos;
use candid::Principal;
use std::marker::PhantomData;
use std::time::Duration;

const MAX_CONCURRENT: usize = 1;
const SLIDING_WINDOW_CALLS: usize = 5;
const SLIDING_WINDOW_DURATION_NS: u64 = Duration::from_millis(60).as_nanos() as u64;

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

pub fn guard_sliding_window(token_id: candid::Nat) -> Result<(), String> {
    let current_time: TimestampNanos = ic_cdk::api::time();

    mutate_state(|s| {
        let call_history = s
            .sliding_window_guards
            .entry(token_id)
            .or_insert_with(Vec::new);

        call_history.retain(|&timestamp| {
            current_time.saturating_sub(timestamp) < SLIDING_WINDOW_DURATION_NS
        });

        if call_history.len() >= SLIDING_WINDOW_CALLS {
            let oldest_call = call_history.first().unwrap();
            let time_until_next_allowed = SLIDING_WINDOW_DURATION_NS - (current_time - oldest_call);
            return Err(format!(
                "Rate limit exceeded. You can make another call in {} milliseconds",
                time_until_next_allowed
            ));
        }

        call_history.push(current_time);

        Ok(())
    })
}

macro_rules! create_permission_guard {
    ($guard_name:ident, $permission:expr, $error_message:expr) => {
        pub fn $guard_name() -> Result<(), String> {
            let caller = ic_cdk::api::msg_caller();
            let has_permission =
                read_state(|state| state.data.permissions.has_permission(&caller, &$permission));

            if has_permission {
                Ok(())
            } else {
                Err($error_message.to_string())
            }
        }
    };
}

create_permission_guard!(
    caller_has_minting_permission,
    Permission::Minting,
    "Caller does not have minting permission"
);

create_permission_guard!(
    caller_has_update_metadata_permission,
    Permission::UpdateMetadata,
    "Caller does not have update metadata permission"
);

create_permission_guard!(
    caller_has_update_collection_metadata_permission,
    Permission::UpdateCollectionMetadata,
    "Caller does not have update collection metadata permission"
);

create_permission_guard!(
    caller_has_manage_authorities_permission,
    Permission::ManageAuthorities,
    "Caller does not have manage authorities permission"
);

create_permission_guard!(
    caller_has_read_uploads_permission,
    Permission::ReadUploads,
    "Caller does not have read uploads permission"
);

create_permission_guard!(
    caller_has_update_uploads_permission,
    Permission::UpdateUploads,
    "Caller does not have update uploads permission"
);
