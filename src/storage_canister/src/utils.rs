use icrc_ledger_types::icrc::generic_value::ICRC3Value as Value;

pub fn get_value_size(value: Value) -> usize {
    match value {
        Value::Blob(blob) => blob.len() as usize,
        Value::Text(text) => text.len() as usize,
        Value::Nat(nat) => nat.0.to_bytes_be().len() as usize,
        Value::Int(int) => int.0.to_bytes_be().1.len() as usize,
        Value::Array(array) => {
            array
                .iter()
                .map(|v| get_value_size(v.clone()))
                .sum()
        }
        Value::Map(map) => {
            map.iter()
                .map(|(k, v)| { k.len() + get_value_size(v.clone()) })
                .sum()
        }
    }
}

#[cfg(test)]
mod tests {}
