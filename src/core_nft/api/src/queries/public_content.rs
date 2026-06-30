pub mod __get_public_entry_test {
    use candid::{CandidType, Nat};
    use core_nft_common::PublicEntry;
    use serde::{Deserialize, Serialize};

    #[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
    pub struct Args {
        pub token_id: Nat,
        pub entry_name: String,
    }

    pub type Response = Result<PublicEntry, String>;
}
