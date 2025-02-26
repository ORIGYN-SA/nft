use crate::{ generate_pocket_query_call, generate_pocket_update_call };

use storage_api_canister::queries::{ get_data, get_storage_size, http_request };
use storage_api_canister::updates::{
    insert_data,
    remove_data,
    update_data,
    init_upload,
    store_chunk,
    finalize_upload,
    cancel_upload,
    delete_file,
};

generate_pocket_query_call!(get_data);
generate_pocket_query_call!(get_storage_size);
generate_pocket_query_call!(http_request);

generate_pocket_update_call!(insert_data);
generate_pocket_update_call!(remove_data);
generate_pocket_update_call!(update_data);
generate_pocket_update_call!(init_upload);
generate_pocket_update_call!(store_chunk);
generate_pocket_update_call!(finalize_upload);
generate_pocket_update_call!(cancel_upload);
generate_pocket_update_call!(delete_file);
