use ic_cdk::export_candid;

pub mod lifecycle;
pub mod queries;
pub mod updates;

pub use lifecycle::*;
pub use queries::*;
pub use updates::*;

export_candid!();
