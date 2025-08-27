use crate::state::read_state;

pub fn caller_is_authorized() -> Result<(), String> {
    if read_state(|state| state.is_caller_authorized()) {
        Ok(())
    } else {
        Err("Caller is not an authorized principal".to_string())
    }
}
