use crate::{generate_pocket_query_call, generate_pocket_update_call};

use index_icrc7_api::types::get_blocks::get_blocks;

// generate_pocket_query_call!(get_blocks);
generate_pocket_update_call!(get_blocks);
