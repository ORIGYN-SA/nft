use super::NftMetadata;
use candid::{ CandidType, Decode, Encode, Nat };
use ic_stable_structures::{ storable::Bound, Storable };
use icrc_ledger_types::{ icrc::generic_value::ICRC3Value as Icrc3Value, icrc1::account::Account };
use serde::{ Deserialize, Serialize };
use tracing::info;
use crate::utils::trace;
use std::collections::HashMap;
use storage_api_canister::types::value_custom::CustomValue as Value;

pub type Icrc7TokenMetadata = HashMap<String, Icrc3Value>;

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct Icrc7Token {
    pub token_id: Nat,
    pub token_name: String,
    pub token_description: Option<String>,
    pub token_logo: Option<String>,
    pub token_owner: Account,
    pub token_metadata: NftMetadata,
}

impl Storable for Icrc7Token {
    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        std::borrow::Cow::Owned(Encode!(self).unwrap())
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl Icrc7Token {
    pub fn new(
        token_id: Nat,
        token_name: String,
        token_description: Option<String>,
        token_logo: Option<String>,
        token_owner: Account
    ) -> Self {
        Self {
            token_id,
            token_name,
            token_logo,
            token_owner,
            token_description,
            token_metadata: NftMetadata::new(),
        }
    }

    pub fn transfer(&mut self, to: Account) {
        self.token_owner = to;
    }

    pub async fn token_metadata(&self) -> Icrc7TokenMetadata {
        let mut metadata = HashMap::<String, Icrc3Value>::new();
        metadata.insert("Name".into(), Icrc3Value::Text(self.token_name.clone()));
        metadata.insert("Symbol".into(), Icrc3Value::Text(self.token_name.clone()));
        if let Some(ref description) = self.token_description {
            metadata.insert("Description".into(), Icrc3Value::Text(description.clone()));
        }
        if let Some(ref logo) = self.token_logo {
            metadata.insert("Logo".into(), Icrc3Value::Text(logo.clone()));
        }

        trace(&format!("nft token_metadata"));

        self.token_metadata
            .get_all_data().await
            .iter()
            .for_each(|(key, value)| {
                trace(&format!("nft token_metadata - key: {:?}, value: {:?}", key, value));
                metadata.insert(key.clone(), value.0.clone());
            });
        metadata
    }

    pub async fn add_metadata(&mut self, metadata: Icrc7TokenMetadata) {
        info!("Adding metadata to token: {:?}", metadata);
        trace(&format!("nft add_metadata"));
        for (key, value) in metadata.iter() {
            self.token_metadata.insert_data(
                self.token_id.clone(),
                key.clone(),
                Value(value.clone())
            ).await;
        }
        trace(&format!("nft add_metadata - finished"));
    }

    fn burn(&mut self) {
        self.token_owner = Account {
            owner: ic_cdk::id(),
            subaccount: None,
        };
    }
}
