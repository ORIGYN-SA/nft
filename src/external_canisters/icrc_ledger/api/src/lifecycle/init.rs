use candid::{CandidType, Principal};
use icrc_ledger_types::{
    icrc::generic_metadata_value::MetadataValue,
    icrc1::{account::Account, transfer::NumTokens},
};

#[derive(CandidType)]
pub struct InitArgs {
    pub token_symbol: String,
    pub token_name: String,
    pub minting_account: Account,
    pub transfer_fee: NumTokens,
    pub metadata: Vec<(String, MetadataValue)>,
    pub initial_balances: Vec<(Account, NumTokens)>,
    pub archive_options: ArchiveOptions,
    pub fee_collector_account: Option<Account>,
}

#[derive(CandidType)]
pub enum LedgerArgument {
    Init(InitArgs),
}

#[derive(CandidType)]
pub struct ArchiveOptions {
    pub trigger_threshold: usize,
    pub num_blocks_to_archive: usize,
    pub controller_id: Principal,
}
