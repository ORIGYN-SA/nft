// This is an experimental feature to generate Rust binding from Candid.
// You may want to manually adjust some of the types.
#![allow(dead_code, unused_imports)]
use candid::{self, CandidType, Deserialize, Principal};
use ic_cdk::api::call::CallResult as Result;

pub type Subaccount = serde_bytes::ByteBuf;
#[derive(CandidType, Deserialize)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Subaccount>,
}
#[derive(CandidType, Deserialize)]
pub enum Value {
    Int(candid::Int),
    Map(Vec<(String, Box<Value>)>),
    Nat(candid::Nat),
    Blob(serde_bytes::ByteBuf),
    Text(String),
    Array(Vec<Box<Value>>),
}
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
#[derive(CandidType, Deserialize)]
pub enum TransferResult {
    Ok(candid::Nat),
    Err(TransferError),
}

pub struct Service(pub Principal);
impl Service {
    pub async fn icrc_7_atomic_batch_transfers(&self) -> Result<(Option<bool>,)> {
        ic_cdk::call(self.0, "icrc7_atomic_batch_transfers", ()).await
    }
    pub async fn icrc_7_balance_of(&self, arg0: Vec<Account>) -> Result<(Vec<candid::Nat>,)> {
        ic_cdk::call(self.0, "icrc7_balance_of", (arg0,)).await
    }
    pub async fn icrc_7_collection_metadata(&self) -> Result<(Vec<(String, Value)>,)> {
        ic_cdk::call(self.0, "icrc7_collection_metadata", ()).await
    }
    pub async fn icrc_7_default_take_value(&self) -> Result<(Option<candid::Nat>,)> {
        ic_cdk::call(self.0, "icrc7_default_take_value", ()).await
    }
    pub async fn icrc_7_description(&self) -> Result<(Option<String>,)> {
        ic_cdk::call(self.0, "icrc7_description", ()).await
    }
    pub async fn icrc_7_logo(&self) -> Result<(Option<String>,)> {
        ic_cdk::call(self.0, "icrc7_logo", ()).await
    }
    pub async fn icrc_7_max_memo_size(&self) -> Result<(Option<candid::Nat>,)> {
        ic_cdk::call(self.0, "icrc7_max_memo_size", ()).await
    }
    pub async fn icrc_7_max_query_batch_size(&self) -> Result<(Option<candid::Nat>,)> {
        ic_cdk::call(self.0, "icrc7_max_query_batch_size", ()).await
    }
    pub async fn icrc_7_max_take_value(&self) -> Result<(Option<candid::Nat>,)> {
        ic_cdk::call(self.0, "icrc7_max_take_value", ()).await
    }
    pub async fn icrc_7_max_update_batch_size(&self) -> Result<(Option<candid::Nat>,)> {
        ic_cdk::call(self.0, "icrc7_max_update_batch_size", ()).await
    }
    pub async fn icrc_7_name(&self) -> Result<(String,)> {
        ic_cdk::call(self.0, "icrc7_name", ()).await
    }
    pub async fn icrc_7_owner_of(&self, arg0: Vec<candid::Nat>) -> Result<(Vec<Option<Account>>,)> {
        ic_cdk::call(self.0, "icrc7_owner_of", (arg0,)).await
    }
    pub async fn icrc_7_permitted_drift(&self) -> Result<(Option<candid::Nat>,)> {
        ic_cdk::call(self.0, "icrc7_permitted_drift", ()).await
    }
    pub async fn icrc_7_supply_cap(&self) -> Result<(Option<candid::Nat>,)> {
        ic_cdk::call(self.0, "icrc7_supply_cap", ()).await
    }
    pub async fn icrc_7_symbol(&self) -> Result<(String,)> {
        ic_cdk::call(self.0, "icrc7_symbol", ()).await
    }
    pub async fn icrc_7_token_metadata(
        &self,
        arg0: Vec<candid::Nat>,
    ) -> Result<(Vec<Option<Vec<(String, Value)>>>,)> {
        ic_cdk::call(self.0, "icrc7_token_metadata", (arg0,)).await
    }
    pub async fn icrc_7_tokens(
        &self,
        arg0: Option<candid::Nat>,
        arg1: Option<candid::Nat>,
    ) -> Result<(Vec<candid::Nat>,)> {
        ic_cdk::call(self.0, "icrc7_tokens", (arg0, arg1)).await
    }
    pub async fn icrc_7_tokens_of(
        &self,
        arg0: Account,
        arg1: Option<candid::Nat>,
        arg2: Option<candid::Nat>,
    ) -> Result<(Vec<candid::Nat>,)> {
        ic_cdk::call(self.0, "icrc7_tokens_of", (arg0, arg1, arg2)).await
    }
    pub async fn icrc_7_total_supply(&self) -> Result<(candid::Nat,)> {
        ic_cdk::call(self.0, "icrc7_total_supply", ()).await
    }
    pub async fn icrc_7_transfer(
        &self,
        arg0: Vec<TransferArg>,
    ) -> Result<(Vec<Option<TransferResult>>,)> {
        ic_cdk::call(self.0, "icrc7_transfer", (arg0,)).await
    }
    pub async fn icrc_7_tx_window(&self) -> Result<(Option<candid::Nat>,)> {
        ic_cdk::call(self.0, "icrc7_tx_window", ()).await
    }
}
