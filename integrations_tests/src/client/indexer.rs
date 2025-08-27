use crate::{generate_pocket_query_call, generate_pocket_update_call};

use index_icrc7::types::get_blocks;

// generate_pocket_query_call!(get_blocks);
generate_pocket_update_call!(get_blocks);
