pub mod get_blocks {
    use crate::index::{IndexType, SortBy};
    use candid::CandidType;
    use icrc_ledger_types::icrc3::blocks::BlockWithId;
    use serde::{Deserialize, Serialize};

    #[derive(CandidType, Deserialize, Serialize)]
    pub struct Args {
        pub start: u64,
        pub length: u64,
        pub filters: Vec<IndexType>,
        pub sort_by: Option<SortBy>,
    }

    #[derive(CandidType, Deserialize, Serialize)]
    pub struct Response {
        pub total: u64,
        pub blocks: Vec<BlockWithId>,
    }
}

pub mod status {
    use candid::CandidType;
    use serde::{Deserialize, Serialize};

    pub type Args = ();

    #[derive(CandidType, Deserialize, Serialize)]
    pub struct Response {
        pub last_block_id: u64,
    }
}

pub mod ledger_id {
    use candid::CandidType;
    use candid::Principal;
    use serde::{Deserialize, Serialize};

    pub type Args = ();

    #[derive(CandidType, Deserialize, Serialize)]
    pub struct Response {
        pub ledger_id: Principal,
    }
}
