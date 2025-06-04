use candid::CandidType;
use ic_stable_structures::{storable::Bound, Storable};
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;
use minicbor::Encoder;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Serialize, Clone, Deserialize, CandidType, Debug, PartialEq, Eq)]
pub struct CustomValue(pub Value);

impl Storable for CustomValue {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buffer = Vec::new();
        minicbor::encode(self, &mut buffer).expect("failed to encode CustomValue");
        Cow::Owned(buffer)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        minicbor::decode(&bytes).expect("failed to decode CustomValue")
    }

    const BOUND: Bound = Bound::Unbounded;
}

impl<C> minicbor::Encode<C> for CustomValue {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match &self.0 {
            Value::Blob(b) => {
                e.array(2)?.u8(0)?.bytes(b)?;
            }
            Value::Text(t) => {
                e.array(2)?.u8(1)?.str(t)?;
            }
            Value::Nat(n) => {
                e.array(2)?
                    .u8(2)?
                    .u64(u64::try_from(n.0.clone()).unwrap())?;
            }
            Value::Int(i) => {
                e.array(2)?
                    .u8(3)?
                    .i64(i64::try_from(i.0.clone()).unwrap())?;
            }
            Value::Array(arr) => {
                e.array(2)?.u8(4)?;
                e.array(arr.len() as u64)?;
                for val in arr {
                    CustomValue(val.clone()).encode(e, _ctx)?;
                }
            }
            Value::Map(map) => {
                e.array(2)?.u8(5)?;
                e.map(map.len() as u64)?;
                for (k, v) in map.iter() {
                    e.str(k)?;
                    CustomValue(v.clone()).encode(e, _ctx)?;
                }
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for CustomValue {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let len = d.array()?.unwrap(); // on attend [tag, data]
        if len != 2 {
            return Err(minicbor::decode::Error::message(
                "expected array of length 2",
            ));
        }

        let tag = d.u8()?;

        let value = match tag {
            0 => {
                let bytes = d.bytes()?.to_vec();
                Value::Blob(bytes.into())
            }
            1 => {
                let s = d.str()?.to_string();
                Value::Text(s)
            }
            2 => {
                let n = d.u64()?;
                Value::Nat(n.into())
            }
            3 => {
                let i = d.i64()?;
                Value::Int(i.into())
            }
            4 => {
                let array_len = d.array()?.unwrap();
                let mut values = Vec::with_capacity(array_len as usize);
                for _ in 0..array_len {
                    let v = CustomValue::decode(d, ctx)?.0;
                    values.push(v);
                }
                Value::Array(values)
            }
            5 => {
                let map_len = d.map()?.unwrap(); // deuxième élément = {k: v, ...}
                let mut map = std::collections::BTreeMap::new();
                for _ in 0..map_len {
                    let k = d.str()?.to_string();
                    let v = CustomValue::decode(d, ctx)?.0;
                    map.insert(k, v);
                }
                Value::Map(map)
            }
            _ => return Err(minicbor::decode::Error::message("invalid tag")),
        };

        Ok(CustomValue(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Int, Nat};
    use minicbor::{decode, encode};
    use serde_bytes::ByteBuf;
    use std::collections::BTreeMap;

    #[test]
    fn test_roundtrip_blob() {
        let original = CustomValue(Value::Blob(ByteBuf::from(vec![1, 2, 3])));
        let mut vec = Vec::new();
        encode(&original, &mut vec).unwrap();
        let decoded: CustomValue = decode(&vec[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_roundtrip_text() {
        let original = CustomValue(Value::Text("hello".to_string()));
        let mut vec = Vec::new();
        encode(&original, &mut vec).unwrap();
        let decoded: CustomValue = decode(&vec[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_roundtrip_nat() {
        let original = CustomValue(Value::Nat(Nat::from(42u64)));
        let mut vec = Vec::new();
        encode(&original, &mut vec).unwrap();
        let decoded: CustomValue = decode(&vec[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_roundtrip_int() {
        let original = CustomValue(Value::Int(Int::from(-42)));
        let mut vec = Vec::new();
        encode(&original, &mut vec).unwrap();
        let decoded: CustomValue = decode(&vec[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_roundtrip_array() {
        let original = CustomValue(Value::Array(vec![
            Value::Text("one".into()),
            Value::Nat(Nat::from(2u64)),
            Value::Int(Int::from(-3)),
        ]));
        let mut vec = Vec::new();
        encode(&original, &mut vec).unwrap();
        let decoded: CustomValue = decode(&vec[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_roundtrip_map() {
        let mut map = BTreeMap::new();
        map.insert("a".to_string(), Value::Nat(Nat::from(1u64)));
        map.insert("b".to_string(), Value::Text("value".to_string()));
        let original = CustomValue(Value::Map(map));
        let mut vec = Vec::new();
        encode(&original, &mut vec).unwrap();
        let decoded: CustomValue = decode(&vec[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_roundtrip_nested() {
        let mut map = BTreeMap::new();
        map.insert("nested".to_string(), Value::Int(Int::from(123)));

        let original = CustomValue(Value::Array(vec![
            Value::Map(map),
            Value::Text("end".to_string()),
        ]));

        let mut vec = Vec::new();
        encode(&original, &mut vec).unwrap();
        let decoded: CustomValue = decode(&vec[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_map_with_array_values() {
        let mut map = BTreeMap::new();
        map.insert(
            "list".to_string(),
            Value::Array(vec![
                Value::Nat(Nat::from(1u64)),
                Value::Text("two".into()),
                Value::Int(Int::from(-3)),
            ]),
        );
        map.insert("text".to_string(), Value::Text("hello".into()));

        let original = CustomValue(Value::Map(map));
        let mut vec = Vec::new();
        encode(&original, &mut vec).unwrap();
        let decoded: CustomValue = decode(&vec[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_array_with_map_elements() {
        let mut inner_map1 = BTreeMap::new();
        inner_map1.insert("a".to_string(), Value::Nat(Nat::from(100u64)));
        inner_map1.insert("b".to_string(), Value::Text("value".into()));

        let mut inner_map2 = BTreeMap::new();
        inner_map2.insert("x".to_string(), Value::Int(Int::from(-42)));

        let original = CustomValue(Value::Array(vec![
            Value::Map(inner_map1),
            Value::Text("middle".into()),
            Value::Map(inner_map2),
        ]));

        let mut vec = Vec::new();
        encode(&original, &mut vec).unwrap();
        let decoded: CustomValue = decode(&vec[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_deeply_nested_structure() {
        let mut inner_map = BTreeMap::new();
        inner_map.insert(
            "level3".to_string(),
            Value::Array(vec![Value::Nat(Nat::from(7u64)), Value::Int(Int::from(-8))]),
        );

        let map_level2 = Value::Map(inner_map);

        let outer_array = Value::Array(vec![
            Value::Text("start".into()),
            map_level2,
            Value::Text("end".into()),
        ]);

        let original = CustomValue(outer_array);
        let mut vec = Vec::new();
        encode(&original, &mut vec).unwrap();
        let decoded: CustomValue = decode(&vec[..]).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_map_with_nested_map_and_array() {
        let mut inner_map = BTreeMap::new();
        inner_map.insert("deep".into(), Value::Int(Int::from(999)));

        let mut map = BTreeMap::new();
        map.insert("simple".into(), Value::Text("test".into()));
        map.insert("nested_map".into(), Value::Map(inner_map.clone()));
        map.insert(
            "nested_array".into(),
            Value::Array(vec![Value::Map(inner_map), Value::Nat(Nat::from(123u64))]),
        );

        let original = CustomValue(Value::Map(map));
        let mut vec = Vec::new();
        encode(&original, &mut vec).unwrap();
        let decoded: CustomValue = decode(&vec[..]).unwrap();
        assert_eq!(original, decoded);
    }
}
