use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    Minting,
    ManageAuthorities,
    UpdateMetadata,
    UpdateCollectionMetadata,
    ReadUploads,
    UpdateUploads,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct PermissionManager {
    pub user_permissions: HashMap<Principal, Vec<Permission>>,
}

impl PermissionManager {
    pub fn new(user_permissions: HashMap<Principal, Vec<Permission>>) -> Self {
        Self { user_permissions }
    }

    pub fn default() -> Self {
        Self {
            user_permissions: HashMap::new(),
        }
    }

    pub fn get_permissions(&self, principal: &Principal) -> Option<&Vec<Permission>> {
        self.user_permissions.get(principal)
    }

    pub fn has_permission(&self, principal: &Principal, permission: &Permission) -> bool {
        self.user_permissions
            .get(principal)
            .map(|permissions| permissions.contains(permission))
            .unwrap_or(false)
    }

    pub fn grant_permission(&mut self, principal: Principal, permission: Permission) {
        self.user_permissions
            .entry(principal)
            .or_insert_with(Vec::new)
            .push(permission);
    }

    pub fn revoke_permission(&mut self, principal: &Principal, permission: &Permission) {
        if let Some(permissions) = self.user_permissions.get_mut(principal) {
            permissions.retain(|p| p != permission);
        }
    }
}
