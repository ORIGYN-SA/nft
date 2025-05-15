// use std::collections::HashMap;

// use candid::{CandidType, Nat, Principal};
// use ic_cdk::api::call::CallResult as Result;
// use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
// use icrc_ledger_types::icrc1::account::Account;
// use serde::{Deserialize, Serialize};

// pub type UpdateInternalResult = Result<Nat>;

// #[derive(CandidType, Serialize, Deserialize, Clone)]
// pub struct UpdateMintingAuthoritiesRequest {
//     pub minting_authorities: Vec<Principal>,
// }

// pub type UpdateMintingAuthoritiesResult = Result<()>;

// #[derive(CandidType, Serialize, Deserialize, Clone)]
// pub struct RemoveMintingAuthoritiesRequest {
//     pub minting_authorities: Vec<Principal>,
// }

// pub type RemoveMintingAuthoritiesResult = Result<()>;

// TODO GWOJDA minting authorities MANAGEMENT
