use bity_ic_icrc3::memory::HeaderRepair;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    DefaultMemoryImpl,
};

/// The ICRC3 block log. Must stay 1: every live collection keeps its blocks
/// there.
const BLOCK_LOG: MemoryId = MemoryId::new(1);
const UPGRADES: MemoryId = MemoryId::new(2);

pub const TOKEN_APPROVALS: MemoryId = MemoryId::new(3);
pub const COLLECTION_APPROVALS: MemoryId = MemoryId::new(4);
pub const METADATA: MemoryId = MemoryId::new(5);

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

pub fn get_metadata_memory() -> VM {
    get_memory(METADATA)
}

/// Hands the ICRC3 block log its memory from this canister's manager, so one
/// manager owns all of stable memory. Call it in `init` and `post_upgrade`
/// before anything creates or deserializes the ICRC3 state.
pub fn init_block_log_memory() {
    bity_ic_icrc3::memory::init_block_log_memory(get_memory(BLOCK_LOG));
}

/// Makes the memory manager header agree with its bucket table before this
/// canister's manager reads it. A collection whose ICRC3 block log ran a
/// second manager over this memory carries a stale bucket count and stale
/// memory sizes. Traps when a memory records more pages than its buckets hold,
/// so the upgrade rolls back. An overlap that the last saved header does not
/// record goes unseen.
pub fn repair_memory_manager_header() -> Option<HeaderRepair> {
    bity_ic_icrc3::memory::repair_memory_manager_header(&DefaultMemoryImpl::default())
        .unwrap_or_else(|error| ic_cdk::trap(&format!("Stable memory is damaged: {error}")))
}

fn get_memory(id: MemoryId) -> VM {
    MEMORY_MANAGER.with(|m| m.get(id))
}
