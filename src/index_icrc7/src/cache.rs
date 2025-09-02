use crate::memory::{get_cache_memory, VM};
use crate::wrapped_values::CustomValue;
use ic_stable_structures::{storable::Bound, StableBTreeMap, Storable};
use std::borrow::Cow;

// Custom struct to replace the tuple for stable storage
#[derive(Clone, Debug)]
pub struct CacheEntry {
    pub timestamp: u64,
    pub value: CustomValue,
}

impl Storable for CacheEntry {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buffer = Vec::new();
        // Encode timestamp as u64 (8 bytes)
        buffer.extend_from_slice(&self.timestamp.to_le_bytes());
        // Encode the CustomValue
        let value_bytes = self.value.to_bytes();
        buffer.extend_from_slice(&value_bytes);
        Cow::Owned(buffer)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let bytes = bytes.as_ref();
        if bytes.len() < 8 {
            panic!("Invalid cache entry bytes: too short");
        }

        // Decode timestamp (first 8 bytes)
        let timestamp = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);

        // Decode the CustomValue (remaining bytes)
        let value_bytes = Cow::Borrowed(&bytes[8..]);
        let value = CustomValue::from_bytes(value_bytes);

        CacheEntry { timestamp, value }
    }

    const BOUND: Bound = Bound::Unbounded;
}

thread_local! {
  pub static __CACHE: std::cell::RefCell<Cache> = std::cell::RefCell::new(init_cache());
}

pub type Cache = StableBTreeMap<u64, CacheEntry, VM>;

pub fn init_cache() -> Cache {
    let memory = get_cache_memory();
    StableBTreeMap::init(memory)
}

pub fn try_get_value(key: u64) -> Option<CustomValue> {
    __CACHE.with(|cache| cache.borrow().get(&key).map(|entry| entry.value.clone()))
}

pub fn try_get_value_with_timestamp(key: u64) -> Option<(u64, CustomValue)> {
    __CACHE.with(|cache| {
        cache
            .borrow()
            .get(&key)
            .map(|entry| (entry.timestamp, entry.value.clone()))
    })
}

pub fn remove_all_values_older_than(timestamp: &u64) -> bool {
    __CACHE.with(|cache| {
        cache
            .borrow()
            .iter()
            .filter(|(_, entry)| entry.timestamp < *timestamp)
            .map(|(key, _)| key)
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|key| {
                cache.borrow_mut().remove(&key);
            });

        true
    })
}

pub fn insert_value(key: u64, value: CustomValue) {
    let timestamp = ic_cdk::api::time();

    let entry = CacheEntry { timestamp, value };
    __CACHE.with(|cache| cache.borrow_mut().insert(key, entry));
}

pub fn remove_value(key: u64) {
    __CACHE.with(|cache| cache.borrow_mut().remove(&key));
}
