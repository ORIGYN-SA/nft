use candid::Principal;
use canister_time::DAY_IN_MS;
use lazy_static::lazy_static;
use pocket_ic::PocketIc;
use rand::{ thread_rng, RngCore };
use types::Cycles;
use types::TimestampMillis;

pub fn random_principal() -> Principal {
    let mut bytes = [0u8; 29];
    thread_rng().fill_bytes(&mut bytes);
    Principal::from_slice(&bytes)
}

pub fn tick_n_blocks(pic: &PocketIc, times: u32) {
    for _ in 0..times {
        pic.tick();
    }
}
pub const T: Cycles = 1_000_000_000_000;
