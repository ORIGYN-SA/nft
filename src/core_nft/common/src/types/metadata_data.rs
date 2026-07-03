use crate::types::value_custom::CustomValue as Value;
use candid::CandidType;
use ic_stable_structures::{storable::Bound, Storable};
use minicbor::{decode, encode, Decode as MinicborDecode, Encode as MinicborEncode};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, CandidType, Clone, Debug)]
pub struct MetadataData {
    pub data: BTreeMap<String, Value>,
}

impl Storable for MetadataData {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let mut buffer = Vec::new();
        encode(self, &mut buffer).expect("failed to encode MetadataData");
        Cow::Owned(buffer)
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut buffer = Vec::new();
        encode(self, &mut buffer).expect("failed to encode MetadataData");
        buffer
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        decode(&bytes).expect("failed to decode MetadataData")
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl<C> MinicborEncode<C> for MetadataData {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.map(self.data.len() as u64)?;
        for (k, v) in &self.data {
            e.str(k)?;
            v.encode(e, _ctx)?;
        }
        Ok(())
    }
}

impl<'b, C> MinicborDecode<'b, C> for MetadataData {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let len = d.map()?.unwrap();
        let mut data = BTreeMap::new();
        for _ in 0..len {
            let k = d.str()?.to_string();
            let v = Value::decode(d, ctx)?;
            data.insert(k, v);
        }
        Ok(MetadataData { data })
    }
}
