use candid::CandidType;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Access rights of a user to a vetKey in [`crate::key_manager::KeyManager`] and/or an encrypted map in [`crate::encrypted_maps::EncryptedMaps`].
#[repr(u8)]
#[derive(
    CandidType,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    strum_macros::FromRepr,
    strum_macros::EnumIter,
)]
pub enum AccessRights {
    /// User can retrieve the vetKey or encrypted map.
    Read = 0,
    /// User can update values in the encrypted map.
    ReadWrite = 1,
    /// User can view/share/revoke access to the vetKey or encrypted map.
    ReadWriteManage = 2,
}

impl Storable for AccessRights {
    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes().into_owned()
    }

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(vec![*self as u8])
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let v = <u8>::from_be_bytes(bytes.as_ref().try_into().unwrap());
        Self::from_repr(v).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 1,
        is_fixed_size: true,
    };
}

pub trait AccessControl:
    CandidType
    + Serialize
    + Clone
    + Copy
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + std::fmt::Debug
    + Storable
    + strum::IntoEnumIterator
{
    /// Returns if the user can read the vetKey or encrypted map.
    fn can_read(&self) -> bool;
    /// Returns if the user can write to the vetKey or encrypted map.
    fn can_write(&self) -> bool;
    /// Returns if the user can view the access rights to the vetKey or encrypted map.
    fn can_get_user_rights(&self) -> bool;
    /// Returns if the user can modify the access rights to the vetKey or encrypted map.
    fn can_set_user_rights(&self) -> bool;
    /// Returns the access rights of the owner of the vetKey or encrypted map.
    fn owner_rights() -> Self;
}

impl AccessControl for AccessRights {
    fn can_read(&self) -> bool {
        matches!(
            self,
            AccessRights::Read | AccessRights::ReadWrite | AccessRights::ReadWriteManage
        )
    }

    fn can_write(&self) -> bool {
        matches!(
            self,
            AccessRights::ReadWrite | AccessRights::ReadWriteManage
        )
    }

    fn can_get_user_rights(&self) -> bool {
        matches!(self, AccessRights::ReadWriteManage)
    }

    fn can_set_user_rights(&self) -> bool {
        matches!(self, AccessRights::ReadWriteManage)
    }

    fn owner_rights() -> Self {
        AccessRights::ReadWriteManage
    }
}
