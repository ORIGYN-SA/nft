use crate::types::value_custom::CustomValue as Value;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as InternalValue;

pub fn get_value_size(value: Value) -> u128 {
    match value.0 {
        InternalValue::Blob(ref blob) => blob.len() as u128,
        InternalValue::Text(ref text) => text.len() as u128,
        InternalValue::Nat(ref nat) => nat.0.to_bytes_be().len() as u128,
        InternalValue::Int(ref int) => int.0.to_bytes_be().1.len() as u128,
        InternalValue::Array(ref array) => {
            array.iter().map(|v| get_value_size(Value(v.clone()))).sum()
        }
        InternalValue::Map(ref map) => map
            .iter()
            .map(|(k, v)| (k.len() as u128) + get_value_size(Value(v.clone())))
            .sum(),
    }
}

#[cfg(test)]
mod tests {}
