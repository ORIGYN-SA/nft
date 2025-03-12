use crate::utils::trace;
use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use storage_api_canister::types::value_custom::CustomValue as Value;

use crate::{
    state::{mutate_state, read_state},
    types::sub_canister::StorageCanister,
};

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Metadata {
    data: HashMap<String, (String, StorageCanister)>,
}

impl Metadata {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn from(metadata: HashMap<String, Value>) -> Self {
        let mut new = Self {
            data: HashMap::new(),
        };

        for (key, value) in metadata.iter() {
            futures::executor::block_on(new.insert_data(None, key.clone(), value.clone()));
        }

        new
    }

    pub async fn insert_data(&mut self, nft_id: Option<Nat>, data_id: String, data: Value) {
        trace(&format!("Inserting data: {:?}", data_id));
        let mut sub_canister_manager = read_state(|state| state.data.sub_canister_manager.clone());

        match sub_canister_manager
            .insert_data(data.clone(), data_id.clone(), nft_id.clone())
            .await
        {
            Ok((hash_id, canister)) => {
                self.data.insert(data_id, (hash_id, canister));
            }
            Err(e) => {
                trace(&format!("Error inserting data: {:?}", e));
                println!("Error inserting data: {:?}", e);
            }
        }

        mutate_state(|state| {
            state.data.sub_canister_manager = sub_canister_manager;
        });
    }

    pub async fn get_data(&self, data_id: String) -> Result<Value, String> {
        let (hash_id, canister) = self.data.get(&data_id).unwrap();
        let data = read_state(|state| state.data.sub_canister_manager.clone());

        data.get_data(canister.clone(), hash_id.clone()).await
    }

    pub async fn get_all_data(&self) -> HashMap<String, Value> {
        let mut all_data = HashMap::new();
        trace(&format!("Getting all data: {:?}", self.data));

        for (data_id, (hash_id, canister)) in self.data.iter() {
            let data = read_state(|state| state.data.sub_canister_manager.clone());

            match data.get_data(canister.clone(), hash_id.clone()).await {
                Ok(value) => {
                    all_data.insert(data_id.to_string(), value);
                }
                Err(e) => {
                    trace(&format!("Error getting data: {:?}", e));
                    println!("Error getting data: {:?}", e);
                }
            }
        }

        all_data
    }

    // pub fn update_data(&mut self, key: String, value: Principal) {
    //     self.data.insert(key, value);
    // }
}
