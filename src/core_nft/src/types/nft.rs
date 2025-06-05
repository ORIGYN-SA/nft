use super::Metadata;
use crate::types::value_custom::CustomValue as Value;
use crate::utils::trace;

use candid::{CandidType, Nat};
use icrc_ledger_types::{icrc::generic_value::ICRC3Value as Icrc3Value, icrc1::account::Account};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Icrc7TokenMetadata = HashMap<String, Icrc3Value>;

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct Icrc7Token {
    pub token_id: Nat,
    pub token_name: String,
    pub token_description: Option<String>,
    pub token_logo: Option<String>,
    pub token_owner: Account,
}

impl Icrc7Token {
    pub fn new(
        token_id: Nat,
        token_name: String,
        token_description: Option<String>,
        token_logo: Option<String>,
        token_owner: Account,
    ) -> Self {
        Self {
            token_id,
            token_name,
            token_logo,
            token_owner,
            token_description,
        }
    }

    pub fn transfer(&mut self, to: Account) {
        self.token_owner = to;
    }

    pub fn token_metadata(&self, tokens_metadata: &Metadata) -> Icrc7TokenMetadata {
        let mut metadata = HashMap::<String, Icrc3Value>::new();
        metadata.insert(
            "icrc7:name".into(),
            Icrc3Value::Text(self.token_name.clone()),
        );
        metadata.insert(
            "icrc7:symbol".into(),
            Icrc3Value::Text(self.token_name.clone()),
        );
        if let Some(ref description) = self.token_description {
            metadata.insert(
                "icrc7:description".into(),
                Icrc3Value::Text(description.clone()),
            );
        }
        if let Some(ref logo) = self.token_logo {
            metadata.insert("icrc7:logo".into(), Icrc3Value::Text(logo.clone()));
        }

        match tokens_metadata.get_all_data(Some(self.token_id.clone())) {
            Ok(data) => {
                for (key, value) in data.iter() {
                    trace(&format!(
                        "nft token_metadata - key: {:?}, value: {:?}",
                        key, value
                    ));
                    let prefixed_key = if !key.starts_with("icrc7:") {
                        format!("icrc7:{}", key)
                    } else {
                        key.clone()
                    };
                    metadata.insert(prefixed_key, value.0.clone());
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

    pub async fn remove_metadata(&mut self, tokens_metadata: &mut Metadata) {
        trace(&format!("nft remove_metadata"));

        tokens_metadata.delete_data(Some(self.token_id.clone()), "metadata".into());

        trace(&format!("nft remove_metadata - finished"));
    }

    pub async fn update_metadata(
        &mut self,
        tokens_metadata: &mut Metadata,
        metadata: Icrc7TokenMetadata,
    ) -> Result<Option<Value>, String> {
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
