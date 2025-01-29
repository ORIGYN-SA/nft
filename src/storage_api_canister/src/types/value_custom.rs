use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use serde::{ Deserialize, Serialize };
use std::borrow::Cow;
use ic_stable_structures::{ storable::Bound, Storable };
use candid::{ CandidType, Decode, Encode };

// THIS IS TEMPORARY. SEE https://forum.dfinity.org/t/add-storage-trait-to-icrc3-value-type/40616
// TODO remove this once the ICRC3Value type implements the Storable trait

#[derive(Serialize, Clone, Deserialize, CandidType, Debug, PartialEq, Eq)]
pub struct CustomValue {
    pub v: Value,
}

impl Storable for CustomValue {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(&self).unwrap())
    }
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(&bytes, Self).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
}
