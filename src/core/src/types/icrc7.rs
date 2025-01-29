// This is an experimental feature to generate Rust binding from Candid.
// You may want to manually adjust some of the types.
#![allow(dead_code, unused_imports)]
use candid::{ self, CandidType, Deserialize, Principal };
use ic_cdk::api::call::CallResult as Result;
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;

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
pub type TransferResult = Result<candid::Nat>;

pub type CollectionMetadataResult = Vec<(String, Value)>;

pub type BalanceOfArgs = Vec<Account>;
pub type BalanceOfResult = Vec<candid::Nat>;

pub type OwnerOfArgs = Vec<candid::Nat>;
pub type OwnerOfResult = Vec<Option<Account>>;

pub type TokensArgs = (Option<candid::Nat>, Option<candid::Nat>);
pub type TokenMetadataResult = Vec<Option<Vec<(String, Value)>>>;

pub type TokenMetadataArgs = Vec<candid::Nat>;
pub type TokensResult = Vec<candid::Nat>;

pub type TokensOfArgs = (Account, Option<candid::Nat>, Option<candid::Nat>);
pub type TokensOfResult = Vec<candid::Nat>;

pub type TransferArgs = Vec<TransferArg>;
pub type TransferResultResult = Vec<Option<TransferResult>>;

pub type AtomicBatchTransfersResult = Option<bool>;

pub type DefaultTakeValueResult = Option<candid::Nat>;

pub type DescriptionResult = Option<String>;

pub type LogoResult = Option<String>;

pub type MaxMemoSizeResult = Option<candid::Nat>;

pub type MaxQueryBatchSizeResult = Option<candid::Nat>;

pub type MaxTakeValueResult = Option<candid::Nat>;

pub type MaxUpdateBatchSizeResult = Option<candid::Nat>;

pub type NameResult = String;

pub type PermittedDriftResult = Option<candid::Nat>;

pub type SupplyCapResult = Option<candid::Nat>;

pub type SymbolResult = String;

pub type TotalSupplyResult = candid::Nat;

pub type TxWindowResult = Option<candid::Nat>;
