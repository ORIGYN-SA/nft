pub fn trace(msg: &str) {
    ic0::debug_print(msg.as_bytes());
}
