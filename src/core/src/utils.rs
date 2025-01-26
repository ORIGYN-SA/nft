use std::hash::{ Hash, Hasher };
use std::collections::hash_map::DefaultHasher;

pub fn hash_string_to_u64(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {}
