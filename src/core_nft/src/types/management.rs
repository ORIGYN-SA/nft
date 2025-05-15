use crate::types::value_custom::CustomValue;

use candid::{CandidType, Nat, Principal};
use ic_cdk::api::call::CallResult as Result;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use storage_api_canister::types::storage::UploadState;

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

pub mod update_authorized_principals {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub authorized_principals: Vec<Principal>,
    }
    pub type Response = Result<()>;
}

pub mod remove_authorized_principals {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub authorized_principals: Vec<Principal>,
    }
    pub type Response = Result<()>;
}

pub mod get_upload_status {
    use super::*;

    pub type Args = String;
    pub type Response = Result<UploadState>;
}

pub mod get_all_uploads {
    use super::*;

    pub type Args0 = Option<Nat>;
    pub type Args1 = Option<Nat>;
    pub type Response = Result<HashMap<String, UploadState>>;
}

pub mod update_collection_metadata {
    use super::*;

    #[derive(CandidType, Serialize, Deserialize, Clone)]
    pub struct Args {
        pub description: Option<String>,
        pub symbol: Option<String>,
        pub name: Option<String>,
        pub logo: Option<String>,
        pub supply_cap: Option<Nat>,
        pub max_query_batch_size: Option<Nat>,
        pub max_update_batch_size: Option<Nat>,
        pub max_take_value: Option<Nat>,
        pub default_take_value: Option<Nat>,
        pub max_memo_size: Option<Nat>,
        pub atomic_batch_transfers: Option<bool>,
        pub tx_window: Option<Nat>,
        pub permitted_drift: Option<Nat>,
        pub max_canister_storage_threshold: Option<Nat>,
        pub collection_metadata: Option<HashMap<String, CustomValue>>,
    }
    pub type Response = Result<()>;
}
