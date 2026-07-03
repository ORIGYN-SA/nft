use crate::memory::get_metadata_memory;
use crate::memory::VM;
use candid::Nat;
use core_nft_common::types::value_custom::CustomValue as Value;
use core_nft_common::types::wrapped_types::WrappedNat;
use core_nft_common::utils::trace;
use core_nft_common::MetadataData;
use ic_stable_structures::StableBTreeMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

thread_local! {
    pub static __METADATA: std::cell::RefCell<Metadata> = std::cell::RefCell::new(init_metadata());
}

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    #[serde(skip, default = "init_btree_map")]
    data: StableBTreeMap<WrappedNat, MetadataData, VM>,
}

impl Clone for Metadata {
    fn clone(&self) -> Self {
        Self {
            data: init_btree_map(),
        }
    }
}

fn init_metadata() -> Metadata {
    Metadata {
        data: init_btree_map(),
    }
}

fn init_btree_map() -> StableBTreeMap<WrappedNat, MetadataData, VM> {
    let memory = get_metadata_memory();
    StableBTreeMap::init(memory)
}

impl Metadata {
    pub fn new() -> Self {
        Self {
            data: init_btree_map(),
        }
    }

    pub fn from(metadata: BTreeMap<String, Value>) -> Self {
        let mut new = Self {
            data: init_btree_map(),
        };

        for (key, value) in metadata.iter() {
            new.insert_data(None, key.clone(), value.clone());
        }

        new
    }

    pub fn insert_data(&mut self, nft_id: Option<Nat>, data_id: String, data: Value) {
        trace(&format!("Inserting data: {:?}", data_id));

        let nat_wrapper = WrappedNat(nft_id.unwrap_or(Nat::from(0u64)));

        let mut metadata_data = if let Some(existing_data) = self.data.get(&nat_wrapper) {
            existing_data.data.clone()
        } else {
            BTreeMap::new()
        };

        metadata_data.insert(data_id, data);

        self.data.insert(
            nat_wrapper,
            MetadataData {
                data: metadata_data,
            },
        );
    }

    pub fn get_data(&self, nft_id: Option<Nat>, data_id: String) -> Result<Value, String> {
        trace(&format!("Getting data: {:?}", data_id));
        let metadata_data = self
            .data
            .get(&WrappedNat(nft_id.unwrap_or(Nat::from(0u64))))
            .ok_or("Data not found".to_string())?;

        match metadata_data
            .data
            .get(&data_id)
            .ok_or("Data not found".to_string())
        {
            Ok(data) => Ok(data.clone()),
            Err(e) => Err(e),
        }
    }

    pub fn get_all_data(&self, nft_id: Option<Nat>) -> Result<BTreeMap<String, Value>, String> {
        trace(&format!("Getting all data for nft: {:?}", nft_id));
        let mut all_data = BTreeMap::new();

        if let Some(nft_id) = nft_id {
            trace(&format!("Getting data for nft: {:?}", nft_id));
            let metadata_data = self
                .data
                .get(&WrappedNat(nft_id))
                .ok_or("Data not found".to_string());
            trace(&format!("Metadata data: {:?}", metadata_data));
            match metadata_data {
                Ok(metadata_data) => {
                    trace(&format!("Metadata data: {:?}", metadata_data));
                    for (key, value) in metadata_data.data.iter() {
                        trace(&format!("Key: {:?}, Value: {:?}", key, value));
                        all_data.insert(key.clone(), value.clone());
                    }
                }
                Err(e) => return Err(e),
            }
        } else {
            for entry in self.data.iter() {
                let metadata_data = entry.value();
                for (key, value) in metadata_data.data.iter() {
                    all_data.insert(key.clone(), value.clone());
                }
            }
        }

        Ok(all_data)
    }

    pub fn get_all_nfts_ids(&self) -> Result<Vec<Nat>, String> {
        trace("Getting all nfts ids");
        let mut all_nfts_ids = Vec::new();

        for entry in self.data.iter() {
            all_nfts_ids.push(entry.key().0.clone());
        }

        Ok(all_nfts_ids)
    }

    pub fn update_data(
        &mut self,
        nft_id: Option<Nat>,
        data_id: String,
        data: Value,
    ) -> Result<Option<Value>, String> {
        trace(&format!("Updating data: {:?}", data_id));
        let metadata_data = self
            .data
            .get(&WrappedNat(nft_id.clone().unwrap_or(Nat::from(0u64))))
            .ok_or("Data not found".to_string())?;

        let mut metadata_data = metadata_data.clone();

        let old_value = metadata_data.data.get(&data_id).cloned();

        metadata_data.data.insert(data_id, data);

        self.data
            .insert(WrappedNat(nft_id.unwrap_or(Nat::from(0u64))), metadata_data);

        trace(&format!("Old value: {:?}", old_value));

        Ok(old_value)
    }

    pub fn delete_data(&mut self, nft_id: Option<Nat>, data_id: String) {
        trace(&format!("Deleting data: {:?}", data_id));
        let mut metadata_data = self
            .data
            .get(&WrappedNat(nft_id.unwrap_or(Nat::from(0u64))))
            .unwrap();

        metadata_data.data.remove(&data_id);
    }

    pub fn replace_all_data(&mut self, nft_id: Option<Nat>, datas: BTreeMap<String, Value>) {
        trace(&format!("Replacing all data for nft: {:?}", nft_id));
        self.data
            .remove(&WrappedNat(nft_id.clone().unwrap_or(Nat::from(0u64))));

        for (key, value) in datas.iter() {
            self.insert_data(nft_id.clone(), key.clone(), value.clone());
        }
    }
}
