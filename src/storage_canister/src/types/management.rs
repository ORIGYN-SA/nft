use std::collections::HashMap;

use candid::{CandidType, Nat, Principal};
use ic_cdk::api::call::CallResult as Result;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct MintRequest {
    pub token_name: String,
    pub token_description: Option<String>,
    pub token_logo: Option<String>,
    pub token_owner: Account,
    pub memo: Option<serde_bytes::ByteBuf>,
}

pub type MintResult = Result<Nat>;

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct UpdateInternalRequest {
    pub token_id: Nat,
    pub token_name: Option<String>,
    pub token_description: Option<String>,
    pub token_logo: Option<String>,
    pub token_metadata: Option<HashMap<String, Value>>,
}

pub type UpdateInternalResult = Result<Nat>;

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct UpdateMintingAuthoritiesRequest {
    pub minting_authorities: Vec<Principal>,
}

pub type UpdateMintingAuthoritiesResult = Result<()>;

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct RemoveMintingAuthoritiesRequest {
    pub minting_authorities: Vec<Principal>,
}

pub type RemoveMintingAuthoritiesResult = Result<()>;
