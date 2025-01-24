use candid::{ Nat, CandidType, Decode, Encode };
use serde::{ Deserialize, Serialize };
use icrc_ledger_types::{ icrc::generic_value::ICRC3Value as Value, icrc1::account::Account };
use ic_stable_structures::{ storable::Bound, Storable };
use std::collections::BTreeMap;

#[derive(CandidType, Serialize, Deserialize, Clone)]
pub struct Icrc7Token {
    pub token_id: Nat,
    pub token_name: String,
    pub token_description: Option<String>,
    pub token_logo: Option<String>,
    pub token_owner: Account,
    pub token_metadata: Icrc7TokenMetadata,
}

pub type Icrc7TokenMetadata = BTreeMap<String, Value>;

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
    fn new(
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
            token_metadata: BTreeMap::new(),
        }
    }

    fn transfer(&mut self, to: Account) {
        self.token_owner = to;
    }

    pub fn token_metadata(&self) -> Icrc7TokenMetadata {
        let mut metadata = BTreeMap::<String, Value>::new();
        metadata.insert("Name".into(), Value::Text(self.token_name.clone()));
        metadata.insert("Symbol".into(), Value::Text(self.token_name.clone()));
        if let Some(ref description) = self.token_description {
            metadata.insert("Description".into(), Value::Text(description.clone()));
        }
        if let Some(ref logo) = self.token_logo {
            metadata.insert("Logo".into(), Value::Text(logo.clone()));
        }
        metadata
    }

    fn burn(&mut self, burn_address: Account) {
        self.token_owner = burn_address;
    }
}
