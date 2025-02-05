use candid::{ Nat, CandidType };
use serde::{ Deserialize, Serialize };
use crate::types::metadata::Metadata;
use std::collections::HashMap;
use storage_api_canister::types::value_custom::CustomValue as Value;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CollectionMetadata(Metadata);

impl CollectionMetadata {
    pub fn new() -> Self {
        Self {
            0: Metadata::new(),
        }
    }

    pub async fn insert_data(&mut self, nft_id: Nat, data_id: String, data: Value) {
        self.0.insert_data(nft_id, data_id, data).await;
    }

    pub async fn get_data(&self, data_id: String) -> Result<Value, String> {
        self.0.get_data(data_id).await
    }

    // pub fn update_data(&mut self, data_id: u64, data: Value) {
    //     self.0.update_data(data_id, data);
    // }

    pub async fn get_all_data(&self) -> HashMap<String, Value> {
        self.0.get_all_data().await
    }
}
