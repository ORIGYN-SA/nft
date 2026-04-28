use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::DefaultMemoryImpl;

pub type VM = VirtualMemory<DefaultMemoryImpl>;

pub fn get_metadata_memory() -> VM {
    // Placeholder - actual implementation in impl crate
    unimplemented!()
}

pub fn get_collection_approvals_memory() -> VM {
    // Placeholder - actual implementation in impl crate
    unimplemented!()
}

pub fn get_token_approvals_memory() -> VM {
    // Placeholder - actual implementation in impl crate
    unimplemented!()
}