use crate::state::mutate_state;
use crate::state::read_state;
use bity_ic_types::TimestampNanos;
use candid::Principal;
use core_nft_common::types::permissions::Permission;
use core_nft_common::{resolve_readers, AccessControl};
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

pub fn caller_is_nft_owner(nft_id: &candid::Nat) -> Result<(), String> {
    let caller = ic_cdk::api::msg_caller();
    read_state(|s| {
        let token = s.data.get_token_by_id(nft_id).ok_or("NFT not found")?;
        if token.token_owner.owner == caller {
            Ok(())
        } else {
            Err("Caller is not the NFT owner".to_string())
        }
    })
}

pub fn caller_is_owner_or_any_reader(nft_id: &candid::Nat) -> Result<(), String> {
    let caller = ic_cdk::api::msg_caller();
    read_state(|s| {
        let token = s.data.get_token_by_id(nft_id).ok_or("NFT not found")?;

        if token.token_owner.owner == caller {
            return Ok(());
        }

        if let Some(private_record) = s.data.private_content_system.nft_private.get(nft_id) {
            for entry in private_record.entries.values() {
                let effective_readers =
                    resolve_readers(&entry.readers, &private_record.default_readers);
                if effective_readers.contains_key(&caller) {
                    return Ok(());
                }
            }
        }

        Err("Caller is not authorized".to_string())
    })
}

pub fn caller_is_owner_or_entry_reader(
    nft_id: &candid::Nat,
    entry_name: &str,
) -> Result<(), String> {
    let caller = ic_cdk::api::msg_caller();
    read_state(|s| {
        let token = s.data.get_token_by_id(nft_id).ok_or("NFT not found")?;

        if token.token_owner.owner == caller {
            return Ok(());
        }

        let private_record = s
            .data
            .private_content_system
            .nft_private
            .get(nft_id)
            .ok_or("Private content not found")?;
        let entry = private_record
            .entries
            .get(entry_name)
            .ok_or("Entry not found")?;

        let effective_readers = resolve_readers(&entry.readers, &private_record.default_readers);
        if effective_readers.contains_key(&caller) {
            return Ok(());
        }

        Err("Caller is not authorized for this entry".to_string())
    })
}

use core_nft_common::PrivateContentStatus;
pub fn entry_content_is_accessible(nft_id: &candid::Nat, entry_name: &str) -> Result<(), String> {
    let caller = ic_cdk::api::msg_caller();
    read_state(|s| {
        let token = s.data.get_token_by_id(nft_id).ok_or("NFT not found")?;
        let is_owner = token.token_owner.owner == caller;

        let private_record = s
            .data
            .private_content_system
            .nft_private
            .get(nft_id)
            .ok_or("Private content not found")?;
        let entry = private_record
            .entries
            .get(entry_name)
            .ok_or("Entry not found")?;

        match &entry.status {
            PrivateContentStatus::Active => Ok(()),
            PrivateContentStatus::PendingUpload => Err("Content is pending upload".to_string()),
            PrivateContentStatus::PendingMinting => Err("Content is pending minting".to_string()),
            PrivateContentStatus::PendingReencryption => {
                if is_owner {
                    Ok(())
                } else {
                    Err("Content is pending re-encryption".to_string())
                }
            }
        }
    })
}

pub fn has_write_access(
    state: &crate::state::RuntimeState,
    token_id: &candid::Nat,
    entry_name: &str,
    caller: candid::Principal,
) -> bool {
    // is caller the owner
    if let Some(token) = state.data.get_token_by_id(token_id) {
        if token.token_owner.owner == caller {
            return true;
        }
    }
    // is caller an authorized reader with write/manage rights
    if let Some(private_record) = state.data.private_content_system.nft_private.get(token_id) {
        if let Some(entry) = private_record.entries.get(entry_name) {
            let effective_readers =
                resolve_readers(&entry.readers, &private_record.default_readers);
            if let Some(reader_info) = effective_readers.get(&caller) {
                return reader_info.rights.can_write();
            }
        }
    }
    false
}
