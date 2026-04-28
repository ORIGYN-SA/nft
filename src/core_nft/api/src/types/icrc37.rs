use ic_stable_structures::StableBTreeMap;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::wrapped_types::{WrappedAccount, WrappedApprovalValue, WrappedNat};
use crate::memory::{get_collection_approvals_memory, get_token_approvals_memory, VM};
use bity_ic_types::TimestampNanos;
use candid::{CandidType, Nat};
use icrc_ledger_types::icrc1::account::{Account, Subaccount};

pub const DEFAULT_MAX_APPROVALS_PER_TOKEN_OR_COLLECTION: usize = 10;

thread_local! {
    pub static __TOKEN_APPROVALS: std::cell::RefCell<TokenApprovals> = std::cell::RefCell::new(init_token_approvals());
    pub static __COLLECTION_APPROVALS: std::cell::RefCell<CollectionApprovals> = std::cell::RefCell::new(init_collection_approvals());
}

#[derive(Encode, Decode, CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Approval {
    #[n(0)]
    pub spender: WrappedAccount,
    #[n(1)]
    pub from: WrappedAccount,
    #[n(2)]
    pub expires_at: Option<TimestampNanos>,
    #[n(3)]
    pub created_at: TimestampNanos,
    #[n(4)]
    pub memo: Option<Vec<u8>>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct ApprovalInfo {
    pub spender: Account,
    pub from_subaccount: Option<Subaccount>,
    pub expires_at: Option<TimestampNanos>,
    pub memo: Option<serde_bytes::ByteBuf>,
    pub created_at_time: TimestampNanos,
}

pub type TokenApprovalValue = HashMap<WrappedAccount, Approval>;
pub type TokenApprovals = StableBTreeMap<WrappedNat, WrappedApprovalValue, VM>;

pub fn init_token_approvals() -> TokenApprovals {
    let memory = get_token_approvals_memory();
    StableBTreeMap::init(memory)
}

// Map to store collection approvals: spender -> approval
pub type CollectionApprovals = StableBTreeMap<WrappedAccount, WrappedApprovalValue, VM>;

pub fn init_collection_approvals() -> CollectionApprovals {
    let memory = get_collection_approvals_memory();
    StableBTreeMap::init(memory)
}

pub mod icrc37_approve_tokens {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct ApproveTokenArg {
        pub token_id: Nat,
        pub approval_info: ApprovalInfo,
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub enum ApproveTokenResult {
        Ok(Nat),
        Err(ApproveTokenError),
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub enum ApproveTokenError {
        InvalidSpender,
        Unauthorized,
        NonExistingTokenId,
        TooOld,
        CreatedInFuture { ledger_time: TimestampNanos },
        GenericError { error_code: Nat, message: String },
        GenericBatchError { error_code: Nat, message: String },
    }

    pub type Args = Vec<ApproveTokenArg>;
    pub type Response = Result<Vec<Option<ApproveTokenResult>>, ApproveTokenError>;
}

pub mod icrc37_approve_collection {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct ApproveCollectionArg {
        pub approval_info: ApprovalInfo,
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub enum ApproveCollectionResult {
        Ok(Nat),
        Err(ApproveCollectionError),
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub enum ApproveCollectionError {
        InvalidSpender,
        TooOld,
        CreatedInFuture { ledger_time: TimestampNanos },
        GenericError { error_code: Nat, message: String },
        GenericBatchError { error_code: Nat, message: String },
    }

    pub type Args = Vec<ApproveCollectionArg>;
    pub type Response = Result<Vec<Option<ApproveCollectionResult>>, ApproveCollectionError>;
}

pub mod icrc37_revoke_token_approvals {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct RevokeTokenApprovalArg {
        pub spender: Option<Account>,
        pub from_subaccount: Option<Subaccount>,
        pub token_id: Nat,
        pub memo: Option<serde_bytes::ByteBuf>,
        pub created_at_time: Option<TimestampNanos>,
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub enum RevokeTokenApprovalResponse {
        Ok(Nat),
        Err(RevokeTokenApprovalError),
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub enum RevokeTokenApprovalError {
        ApprovalDoesNotExist,
        Unauthorized,
        NonExistingTokenId,
        TooOld,
        CreatedInFuture { ledger_time: TimestampNanos },
        GenericError { error_code: Nat, message: String },
        GenericBatchError { error_code: Nat, message: String },
    }

    pub type Args = Vec<RevokeTokenApprovalArg>;
    pub type Response = Result<Vec<Option<RevokeTokenApprovalResponse>>, RevokeTokenApprovalError>;
}

pub mod icrc37_revoke_collection_approvals {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct RevokeCollectionApprovalArg {
        pub spender: Option<Account>,
        pub from_subaccount: Option<Subaccount>,
        pub memo: Option<serde_bytes::ByteBuf>,
        pub created_at_time: Option<TimestampNanos>,
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub enum RevokeCollectionApprovalResult {
        Ok(Nat),
        Err(RevokeCollectionApprovalError),
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub enum RevokeCollectionApprovalError {
        ApprovalDoesNotExist,
        TooOld,
        CreatedInFuture { ledger_time: TimestampNanos },
        GenericError { error_code: Nat, message: String },
        GenericBatchError { error_code: Nat, message: String },
    }

    pub type Args = Vec<RevokeCollectionApprovalArg>;
    pub type Response =
        Result<Vec<Option<RevokeCollectionApprovalResult>>, RevokeCollectionApprovalError>;
}

pub mod icrc37_transfer_from {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct TransferFromArg {
        pub spender_subaccount: Option<Subaccount>,
        pub from: Account,
        pub to: Account,
        pub token_id: Nat,
        pub memo: Option<serde_bytes::ByteBuf>,
        pub created_at_time: Option<TimestampNanos>,
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub enum TransferFromResult {
        Ok(Nat),
        Err(TransferFromError),
    }

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub enum TransferFromError {
        InvalidRecipient,
        Unauthorized,
        NonExistingTokenId,
        TooOld,
        CreatedInFuture { ledger_time: TimestampNanos },
        Duplicate { duplicate_of: Nat },
        GenericError { error_code: Nat, message: String },
        GenericBatchError { error_code: Nat, message: String },
    }

    pub type Args = Vec<TransferFromArg>;
    pub type Response = Result<Vec<Option<TransferFromResult>>, TransferFromError>;
}

pub mod icrc37_get_token_approvals {
    use super::*;
    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct TokenApproval {
        pub token_id: Nat,
        pub approval_info: ApprovalInfo,
    }

    pub type Args0 = Nat;
    pub type Args1 = Option<TokenApproval>;
    pub type Args2 = Option<Nat>;
    pub type Response = Vec<TokenApproval>;
}

pub mod icrc37_get_collection_approvals {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct CollectionApproval {
        pub approval_info: ApprovalInfo,
    }

    pub type Args0 = Account;
    pub type Args1 = Option<CollectionApproval>;
    pub type Args2 = Option<Nat>;
    pub type Response = Vec<CollectionApproval>;
}

pub mod icrc37_is_approved {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct IsApprovedArg {
        pub spender: Account,
        pub from_subaccount: Option<Subaccount>,
        pub token_id: Nat,
    }

    pub type Args = Vec<IsApprovedArg>;
    pub type Response = Vec<bool>;
}

pub mod icrc37_max_approvals_per_token_or_collection {
    use super::*;
    pub type Args = ();
    pub type Response = Option<Nat>;
}

pub mod icrc37_max_revoke_approvals {
    use super::*;
    pub type Args = ();
    pub type Response = Option<Nat>;
}
