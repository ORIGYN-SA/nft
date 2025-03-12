use self::setup::{TestEnv, TestEnvBuilder};

pub mod setup;
pub mod setup_core;

pub fn default_test_setup() -> TestEnv {
    TestEnvBuilder::new().build()
}
