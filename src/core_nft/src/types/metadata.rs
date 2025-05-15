use crate::memory::VM;
use crate::types::value_custom::CustomValue as Value;
use crate::{memory::get_metadata_nft_memory, utils::trace};

use candid::{CandidType, Decode, Encode, Nat};
use ic_stable_structures::{storable::Bound, StableBTreeMap, Storable};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct MetadataData {
    pub data: HashMap<String, Value>,
}

impl Storable for MetadataData {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(&bytes, Self).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(Serialize, Deserialize, CandidType, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NatWrapper(Nat);

impl Storable for NatWrapper {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(&bytes, Self).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    #[serde(skip, default = "init_storage_raw")]
    data: StableBTreeMap<NatWrapper, MetadataData, VM>,
}

impl Clone for Metadata {
    fn clone(&self) -> Self {
        Self {
            data: init_storage_raw(),
        }
    }
}

fn init_storage_raw() -> StableBTreeMap<NatWrapper, MetadataData, VM> {
    let memory = get_metadata_nft_memory();
    StableBTreeMap::init(memory)
}

impl Metadata {
    pub fn new() -> Self {
        Self {
            data: init_storage_raw(),
        }
    }

    pub fn from(metadata: HashMap<String, Value>) -> Self {
        let mut new = Self {
            data: init_storage_raw(),
        };

        for (key, value) in metadata.iter() {
            new.insert_data(None, key.clone(), value.clone());
        }

        new
    }

    pub fn insert_data(&mut self, nft_id: Option<Nat>, data_id: String, data: Value) {
        trace(&format!("Inserting data: {:?}", data_id));

        let nat_wrapper = NatWrapper(nft_id.unwrap_or(Nat::from(0u64)));

        let mut metadata_data = if let Some(existing_data) = self.data.get(&nat_wrapper) {
            existing_data.data.clone()
        } else {
            HashMap::new()
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
            .get(&NatWrapper(nft_id.unwrap_or(Nat::from(0u64))))
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

    pub fn get_all_data(&self, nft_id: Option<Nat>) -> Result<HashMap<String, Value>, String> {
        trace(&format!("Getting all data for nft: {:?}", nft_id));
        let mut all_data = HashMap::new();

        if let Some(nft_id) = nft_id {
            trace(&format!("Getting data for nft: {:?}", nft_id));
            let metadata_data = self
                .data
                .get(&NatWrapper(nft_id))
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
            for (_, metadata_data) in self.data.iter() {
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

        for (key, _) in self.data.iter() {
            all_nfts_ids.push(key.0.clone());
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
            .get(&NatWrapper(nft_id.clone().unwrap_or(Nat::from(0u64))))
            .ok_or("Data not found".to_string())?;

        let mut metadata_data = metadata_data.clone();

        let old_value = metadata_data.data.get(&data_id).cloned();

        metadata_data.data.insert(data_id, data);

        self.data
            .insert(NatWrapper(nft_id.unwrap_or(Nat::from(0u64))), metadata_data);

        trace(&format!("Old value: {:?}", old_value));

        Ok(old_value)
    }

    pub fn delete_data(&mut self, nft_id: Option<Nat>, data_id: String) {
        trace(&format!("Deleting data: {:?}", data_id));
        let mut metadata_data = self
            .data
            .get(&NatWrapper(nft_id.unwrap_or(Nat::from(0u64))))
            .unwrap();

        metadata_data.data.remove(&data_id);
    }

    pub fn erase_all_data(&mut self, nft_id: Option<Nat>, datas: HashMap<String, Value>) {
        trace(&format!("Erasing all data for nft: {:?}", nft_id));
        self.data
            .remove(&NatWrapper(nft_id.clone().unwrap_or(Nat::from(0u64))));

        for (key, value) in datas.iter() {
            self.insert_data(nft_id.clone(), key.clone(), value.clone());
        }
    }
}
