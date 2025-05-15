// This is an experimental feature to generate Rust binding from Candid.
// You may want to manually adjust some of the types.
#![allow(dead_code, unused_imports)]
use candid::{self, CandidType, Deserialize, Principal};
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use icrc_ledger_types::icrc1::account::Account;

pub const DEFAULT_TX_WINDOW: u64 = 10_000_000_000; // 10 seconds
pub const DEFAULT_PERMITTED_DRIFT: u64 = 50_000_000; // 50ms
pub const DEFAULT_TAKE_VALUE: usize = 100;
pub const DEFAULT_MAX_UPDATE_BATCH_SIZE: u128 = 100;
pub const DEFAULT_MAX_SUPPLY_CAP: u128 = 10_000;
pub const DEFAULT_MAX_MEMO_SIZE: u128 = 1_000_000;

#[derive(CandidType, Deserialize)]
pub struct TransferArg {
    pub to: Account,
    pub token_id: candid::Nat,
    pub memo: Option<serde_bytes::ByteBuf>,
    pub from_subaccount: Option<serde_bytes::ByteBuf>,
    pub created_at_time: Option<u64>,
}
#[derive(CandidType, Deserialize)]
pub enum TransferError {
    GenericError {
        message: String,
        error_code: candid::Nat,
    },
    Duplicate {
        duplicate_of: candid::Nat,
    },
    NonExistingTokenId,
    Unauthorized,
    CreatedInFuture {
        ledger_time: u64,
    },
    InvalidRecipient,
    GenericBatchError {
        message: String,
        error_code: candid::Nat,
    },
    TooOld,
}

pub mod icrc7_transfer {
    use candid::Nat;
    use serde::Serialize;
    #[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
    pub enum TransferError {
        NonExistingTokenId,
        InvalidRecipient,
        Unauthorized,
        TooOld,
        CreatedInFuture { ledger_time: Nat },
        Duplicate { duplicate_of: Nat },
        GenericError { error_code: Nat, message: String },
        GenericBatchError { error_code: Nat, message: String },
    }

    use super::*;
    pub type Response = Vec<Option<Result<Nat, TransferError>>>;
    pub type Args = Vec<TransferArg>;
}

pub mod icrc7_collection_metadata {
    use super::*;
    pub type Args = ();
    pub type Response = Vec<(String, Value)>;
}

pub mod icrc7_balance_of {
    use super::*;
    pub type Response = Vec<candid::Nat>;
    pub type Args = Vec<Account>;
}

pub mod icrc7_owner_of {
    use super::*;
    pub type Args = Vec<candid::Nat>;
    pub type Response = Vec<Option<Account>>;
}

pub mod icrc7_tokens {
    use super::*;
    pub type Args0 = Option<candid::Nat>;
    pub type Args1 = Option<candid::Nat>;
    pub type Response = Vec<candid::Nat>;
}

pub mod icrc7_token_metadata {
    use super::*;
    pub type Args = Vec<candid::Nat>;
    pub type Response = Vec<Option<Vec<(String, Value)>>>;
}

pub mod icrc7_tokens_of {
    use super::*;
    pub type Args0 = Account;
    pub type Args1 = Option<candid::Nat>;
    pub type Args2 = Option<candid::Nat>;
    pub type Response = Vec<candid::Nat>;
}

pub mod icrc7_atomic_batch_transfers {
    use super::*;
    pub type Args = ();
    pub type Response = Option<bool>;
}

pub mod icrc7_default_take_value {
    use super::*;
    pub type Args = ();
    pub type Response = Option<candid::Nat>;
}

pub mod icrc7_max_memo_size {
    use super::*;
    pub type Args = ();
    pub type Response = Option<candid::Nat>;
}

pub mod icrc7_description {
    use super::*;
    pub type Args = ();
    pub type Response = Option<String>;
}

pub mod icrc7_logo {
    use super::*;
    pub type Args = ();
    pub type Response = Option<String>;
}

pub mod icrc7_max_query_batch_size {
    use super::*;
    pub type Args = ();
    pub type Response = Option<candid::Nat>;
}

pub mod icrc7_max_take_value {
    use super::*;
    pub type Args = ();
    pub type Response = Option<candid::Nat>;
}

pub mod icrc7_max_update_batch_size {
    use super::*;
    pub type Args = ();
    pub type Response = Option<candid::Nat>;
}

pub mod icrc7_name {
    use super::*;
    pub type Args = ();
    pub type Response = String;
}

pub mod icrc7_permitted_drift {
    use super::*;
    pub type Args = ();
    pub type Response = Option<candid::Nat>;
}

pub mod icrc7_supply_cap {
    use super::*;
    pub type Args = ();
    pub type Response = Option<candid::Nat>;
}

pub mod icrc7_symbol {
    use super::*;
    pub type Args = ();
    pub type Response = String;
}

pub mod icrc7_total_supply {
    use super::*;
    pub type Args = ();
    pub type Response = candid::Nat;
}

pub mod icrc7_tx_window {
    use super::*;
    pub type Args = ();
    pub type Response = Option<candid::Nat>;
}
