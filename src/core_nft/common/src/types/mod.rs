pub mod access_control;
pub mod http;
pub mod icrc37;
pub mod icrc7;
pub mod management;
pub mod metadata_data;
pub mod permissions;
pub mod private_content;
pub mod public_content;
pub mod sub_canister;
pub mod value_custom;
pub mod wrapped_types;

pub use access_control::*;
pub use icrc37::*;
pub use icrc7::*;
pub use management::*;
pub use metadata_data::*;
pub use permissions::*;
pub use private_content::*;
pub use public_content::*;
pub use sub_canister::*;
pub use value_custom::*;
pub use wrapped_types::*;

pub type Sha256Hash = [u8; 32];

pub const MAX_CONTENT_SIZE: u64 = 100 * 1024 * 1024; // 100MB
                                                     // pub const MAX_PUBLIC_CONTENT_SIZE: u64 = 100 * 1024 * 1024; // 100MB
pub const CHUNK_SIZE: usize = 1024 * 1024; // 1MB
