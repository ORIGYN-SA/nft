use icrc_ledger_types::icrc3::archive::ICRC3ArchiveInfo;
use icrc_ledger_types::icrc3::blocks::{GetBlocksRequest, GetBlocksResult};

pub mod icrc3_get_archives {
    use super::*;
    pub type Args = ();
    pub type Response = Vec<ICRC3ArchiveInfo>;
}

pub mod icrc3_get_blocks {
    use super::*;
    pub type Args = Vec<GetBlocksRequest>;
    pub type Response = GetBlocksResult;
}

pub mod icrc3_get_properties {
    pub use bity_ic_icrc3::types::icrc3_get_properties::{Args, Response};
}

pub mod icrc3_get_tip_certificate {
    pub use bity_ic_icrc3::types::icrc3_get_tip_certificate::{Args, Response};
}

pub mod icrc3_supported_block_types {
    pub use bity_ic_icrc3::types::icrc3_supported_block_types::{Args, Response};
}
