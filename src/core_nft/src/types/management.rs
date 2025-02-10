use std::collections::HashMap;

use candid::{CandidType, Nat, Principal};
use ic_cdk::api::call::CallResult as Result;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};

pub mod mint {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub token_name: String,
        pub token_description: Option<String>,
        pub token_logo: Option<String>,
        pub token_owner: Account,
        pub memo: Option<serde_bytes::ByteBuf>,
    }
    pub type Response = Result<Nat>;
}

pub mod update_nft_metadata {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub token_id: Nat,
        pub token_name: Option<String>,
        pub token_description: Option<String>,
        pub token_logo: Option<String>,
        pub token_metadata: Option<HashMap<String, Value>>,
    }
    pub type Response = Result<Nat>;
}

pub mod update_minting_authorities {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub minting_authorities: Vec<Principal>,
    }
    pub type Response = Result<()>;
}

pub mod remove_minting_authorities {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub minting_authorities: Vec<Principal>,
    }
    pub type Response = Result<()>;
}
