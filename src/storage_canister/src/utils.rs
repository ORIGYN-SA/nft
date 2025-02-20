pub fn trace(msg: &str) {
    unsafe {
        ic0::debug_print(msg.as_ptr() as i32, msg.len() as i32);
    }
}

#[cfg(test)]
mod tests {}
