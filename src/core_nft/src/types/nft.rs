use candid::{CandidType, Nat};
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct Icrc7Token {
    pub token_id: Nat,
    pub token_name: String,
    pub token_metadata_url: String,
    pub token_owner: Account,
}

impl Icrc7Token {
    pub fn new(
        token_id: Nat,
        token_name: String,
        token_metadata_url: Url,
        token_owner: Account,
    ) -> Self {
        Self {
            token_id,
            token_name,
            token_metadata_url: token_metadata_url.to_string(),
            token_owner,
        }
    }

    pub fn transfer(&mut self, to: Account) {
        self.token_owner = to;
    }
}
