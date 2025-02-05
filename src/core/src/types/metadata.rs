use candid::Nat;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use storage_api_canister::types::value_custom::CustomValue as Value;

use crate::{ state::{ mutate_state, read_state }, sub_canister_manager::Canister };

#[derive(Serialize, Deserialize, Clone)]
pub struct Metadata {
    data: HashMap<Nat, (String, Canister)>,
}

impl Metadata {
    pub async fn insert_data(&mut self, nft_id: Nat, data_id: Nat, data: Value) {
        let mut sub_canister_manager = read_state(|state| state.data.sub_canister_manager.clone());

        match sub_canister_manager.insert_data(data.clone(), data_id.clone(), nft_id.clone()).await {
            Ok((hash_id, canister)) => {
                self.data.insert(data_id, (hash_id, canister));
            }
            Err(e) => {
                println!("Error inserting data: {:?}", e);
            }
        }

        mutate_state(|state| {
            state.data.sub_canister_manager = sub_canister_manager;
        });
    }

    pub async fn get_data(&self, data_id: Nat) -> Result<Value, String> {
        let (hash_id, canister) = self.data.get(&data_id).unwrap();
        let data = read_state(|state| state.data.sub_canister_manager.clone());

        data.get_data(canister.clone(), hash_id.clone()).await
    }

    // pub fn update_data(&mut self, key: String, value: Principal) {
    //     self.data.insert(key, value);
    // }
}
