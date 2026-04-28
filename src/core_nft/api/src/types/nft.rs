use crate::types::Metadata;
use crate::utils::trace;

use crate::types::value_custom::CustomValue as Value;
use candid::{CandidType, Nat};
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Icrc3Value;
use icrc_ledger_types::icrc1::account::Account;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct Icrc7Token {
    pub token_id: Nat,
    pub token_owner: Account,
}

pub type Icrc7TokenMetadata = Vec<(String, Icrc3Value)>;

impl Icrc7Token {
    pub fn new(token_id: Nat, token_owner: Account) -> Self {
        Self {
            token_id,
            token_owner,
        }
    }

    pub fn transfer(&mut self, to: Account) {
        self.token_owner = to;
    }

    pub fn token_metadata(&self, tokens_metadata: &Metadata) -> Icrc7TokenMetadata {
        let mut metadata = Vec::new();
        match tokens_metadata.get_all_data(Some(self.token_id.clone())) {
            Ok(data) => {
                for (key, value) in data.iter() {
                    metadata.push((key.clone(), value.0.clone()));
                }
            }
            Err(e) => {
                trace(&format!("nft token_metadata - error: {:?}", e));
            }
        }

        metadata
    }

    pub fn add_metadata(&mut self, tokens_metadata: &mut Metadata, metadata: Icrc7TokenMetadata) {
        trace(&format!("nft add_metadata"));

        for (key, value) in metadata.iter() {
            trace(&format!(
                "nft add_metadata - key: {:?}, value: {:?}",
                key, value
            ));
            tokens_metadata.insert_data(
                Some(self.token_id.clone()),
                key.clone(),
                Value(value.clone()),
            );
        }

        trace(&format!("nft add_metadata - finished"));
    }

    pub fn replace_metadata(
        &mut self,
        tokens_metadata: &mut Metadata,
        metadata: Icrc7TokenMetadata,
    ) {
        trace(&format!("nft replace_metadata"));

        tokens_metadata.replace_all_data(
            Some(self.token_id.clone()),
            metadata.into_iter().map(|(k, v)| (k, Value(v))).collect(),
        );

        trace(&format!("nft replace_metadata - finished"));
    }

    pub async fn remove_metadata(&mut self, tokens_metadata: &mut Metadata) {
        trace(&format!("nft remove_metadata"));

        tokens_metadata.delete_data(Some(self.token_id.clone()), "metadata".into());

        trace(&format!("nft remove_metadata - finished"));
    }

    pub async fn update_metadata(
        &mut self,
        tokens_metadata: &mut Metadata,
        metadata: Icrc7TokenMetadata,
    ) -> Result<Option<Icrc3Value>, String> {
        trace(&format!("nft update_metadata"));

        for (key, value) in metadata.iter() {
            tokens_metadata
                .update_data(
                    Some(self.token_id.clone()),
                    key.clone(),
                    Value(value.clone()),
                )
                .unwrap();
        }

        trace(&format!("nft update_metadata - finished"));

        Ok(None)
    }
}
