use ic_cdk_timers::TimerId;
use std::time::Duration;

use types::{Milliseconds, TimestampMillis, TimestampNanos};

pub const SECOND_IN_MS: Milliseconds = 1000;
pub const MINUTE_IN_MS: Milliseconds = SECOND_IN_MS * 60;
pub const HOUR_IN_MS: Milliseconds = MINUTE_IN_MS * 60;
pub const DAY_IN_MS: Milliseconds = HOUR_IN_MS * 24;
pub const WEEK_IN_MS: Milliseconds = DAY_IN_MS * 7;

pub const NANOS_PER_MILLISECOND: u64 = 1_000_000;

pub fn timestamp_seconds() -> u64 {
    timestamp_nanos() / 1_000_000_000
}

pub fn timestamp_millis() -> u64 {
    timestamp_nanos() / 1_000_000
}

pub fn timestamp_micros() -> u64 {
    timestamp_nanos() / 1_000
}

#[cfg(target_arch = "wasm32")]
pub fn timestamp_nanos() -> u64 {
    unsafe { ic0::time() as u64 }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn timestamp_nanos() -> u64 {
    use std::time::SystemTime;

    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

pub fn now_millis() -> TimestampMillis {
    now_nanos() / NANOS_PER_MILLISECOND
}

#[cfg(target_arch = "wasm32")]
pub fn now_nanos() -> TimestampNanos {
    ic_cdk::api::time()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn now_nanos() -> TimestampNanos {
    0
}

pub fn run_now_then_interval(interval: Duration, func: fn()) -> TimerId {
    ic_cdk_timers::set_timer(Duration::ZERO, func);
    ic_cdk_timers::set_timer_interval(interval, func)
}

pub fn run_interval(interval: Duration, func: fn()) {
    ic_cdk_timers::set_timer_interval(interval, func);
}

pub fn run_once(func: fn()) {
    ic_cdk_timers::set_timer(Duration::ZERO, func);
}
