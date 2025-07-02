use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    DefaultMemoryImpl,
};

const UPGRADES: MemoryId = MemoryId::new(0);

pub const TOKEN_APPROVALS: MemoryId = MemoryId::new(1);
pub const COLLECTION_APPROVALS: MemoryId = MemoryId::new(2);

pub type VM = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: MemoryManager<DefaultMemoryImpl> = MemoryManager::init(
        DefaultMemoryImpl::default()
    );
}

pub fn get_token_approvals_memory() -> VM {
    get_memory(TOKEN_APPROVALS)
}

pub fn get_collection_approvals_memory() -> VM {
    get_memory(COLLECTION_APPROVALS)
}

pub fn get_upgrades_memory() -> VM {
    get_memory(UPGRADES)
}

fn get_memory(id: MemoryId) -> VM {
    MEMORY_MANAGER.with(|m| m.get(id))
}
