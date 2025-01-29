use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;

pub fn get_value_size(value: Value) -> u128 {
    match value {
        Value::Blob(blob) => blob.len() as u128,
        Value::Text(text) => text.len() as u128,
        Value::Nat(nat) => nat.0.to_bytes_be().len() as u128,
        Value::Int(int) => int.0.to_bytes_be().1.len() as u128,
        Value::Array(array) => {
            array
                .iter()
                .map(|v| get_value_size(v.clone()))
                .sum()
        }
        Value::Map(map) => {
            map.iter()
                .map(|(k, v)| { (k.len() as u128) + get_value_size(v.clone()) })
                .sum()
        }
    }
}

#[cfg(test)]
mod tests {}
