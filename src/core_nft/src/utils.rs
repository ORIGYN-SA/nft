use crate::types::value_custom::CustomValue as Value;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as InternalValue;

use crate::state::read_state;
use crate::types::icrc7;
use candid::Nat;

pub fn check_memo(memo: Option<serde_bytes::ByteBuf>) -> Result<(), String> {
    if let Some(ref memo) = memo {
        let max_memo_size: usize = usize::try_from(
            read_state(|state| {
                state
                    .data
                    .max_memo_size
                    .clone()
                    .unwrap_or(Nat::from(icrc7::DEFAULT_MAX_MEMO_SIZE))
            })
            .0,
        )
        .unwrap();

        trace(&format!("Memo Size: {}", memo.len()));
        trace(&format!("Max Memo Size: {}", max_memo_size));

        if memo.len() > max_memo_size {
            trace("Exceeds Max Memo Size");
            return Err("Exceeds Max Memo Size".to_string());
        }
    }
    Ok(())
}

pub fn trace(msg: &str) {
    unsafe {
        ic0::debug_print(msg.as_ptr() as i32, msg.len() as i32);
    }
}

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
