use crate::state::icrc3_get_archives as icrc3_get_archives_impl;
use crate::state::icrc3_get_blocks as icrc3_get_blocks_impl;
use crate::state::icrc3_get_properties as icrc3_get_properties_impl;
use crate::state::icrc3_get_tip_certificate as icrc3_get_tip_certificate_impl;
use crate::state::icrc3_supported_block_types as icrc3_supported_block_types_impl;

use crate::types::transaction::Icrc3Transaction;

use ic_cdk::query;

pub use crate::types::icrc3::{
    icrc3_get_archives::{Args as GetArchivesArg, Response as GetArchivesResponse},
    icrc3_get_blocks::{Args as GetBlocksArg, Response as GetBlocksResponse},
    icrc3_get_properties::{Args as GetArchivePropsArg, Response as GetArchivePropsResponse},
    icrc3_get_tip_certificate::{
        Args as GetTipCertificateArg, Response as GetTipCertificateResponse,
    },
    icrc3_supported_block_types::{
        Args as GetSupportedBlockTypesArg, Response as GetSupportedBlockTypesResponse,
    },
};
pub use icrc_ledger_types::icrc3::blocks::GetBlocksResult;

#[query]
async fn icrc3_get_archives(_: GetArchivesArg) -> GetArchivesResponse {
    icrc3_get_archives_impl::<Icrc3Transaction>()
}

#[query]
async fn icrc3_get_blocks(args: GetBlocksArg) -> GetBlocksResult {
    icrc3_get_blocks_impl::<Icrc3Transaction>(args).await
}

#[query]
async fn icrc3_get_properties(_: GetArchivePropsArg) -> GetArchivePropsResponse {
    icrc3_get_properties_impl::<Icrc3Transaction>()
}

#[query]
async fn icrc3_get_tip_certificate(_: GetTipCertificateArg) -> GetTipCertificateResponse {
    icrc3_get_tip_certificate_impl::<Icrc3Transaction>()
}

#[query]
async fn icrc3_supported_block_types(
    _: GetSupportedBlockTypesArg,
) -> GetSupportedBlockTypesResponse {
    icrc3_supported_block_types_impl::<Icrc3Transaction>()
}
