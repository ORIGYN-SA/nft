use crate::{generate_pocket_query_call, generate_pocket_update_call};

use storage_api_canister::queries::{get_storage_size, http_request};
use storage_api_canister::updates::{cancel_upload, finalize_upload, init_upload, store_chunk};

generate_pocket_query_call!(get_storage_size);
generate_pocket_query_call!(http_request);

generate_pocket_update_call!(init_upload);
generate_pocket_update_call!(store_chunk);
generate_pocket_update_call!(finalize_upload);
generate_pocket_update_call!(cancel_upload);
