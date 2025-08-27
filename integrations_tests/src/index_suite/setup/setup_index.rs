use crate::wasms::INDEX_WASM;

use candid::encode_one;
use candid::types::value::IDLValue;
use candid::Principal;
use index_icrc7::lifecycle::Args;
use pocket_ic::PocketIc;

#[derive(Clone, Debug, Default, candid::CandidType, candid::Deserialize)]
pub struct RegistryCanisterInitPayload {
    pub mutations: Vec<u64>,
    // pub mutations: Vec<RegistryAtomicMutateRequest>, this is the real type, but we don't need it for now as we init with empty vec
}

pub fn setup_index_canister(
    pic: &mut PocketIc,
    core_canister_id: Principal,
    args: Args,
    controller: Principal,
) -> Principal {
    let index_wasm = INDEX_WASM.clone();
    pic.add_cycles(core_canister_id, 100_000_000_000_000_000_000);

    pic.set_controllers(
        core_canister_id,
        Some(controller.clone()),
        vec![controller.clone()],
    )
    .unwrap();
    pic.tick();

    println!(
        "ret IDLValue {:?}",
        IDLValue::try_from_candid_type(&&args).unwrap()
    );

    pic.install_canister(
        core_canister_id,
        index_wasm,
        encode_one(args).unwrap(),
        Some(controller.clone()),
    );

    core_canister_id
}

pub fn upgrade_index_canister(
    pic: &mut PocketIc,
    core_canister_id: Principal,
    args: Args,
    controller: Principal,
) {
    let index_wasm = INDEX_WASM.clone();
    pic.add_cycles(core_canister_id, 100_000_000_000_000_000_000);

    pic.set_controllers(
        core_canister_id,
        Some(controller.clone()),
        vec![controller.clone()],
    )
    .unwrap();
    pic.tick();

    pic.upgrade_canister(
        core_canister_id,
        index_wasm,
        encode_one(args).unwrap(),
        Some(controller.clone()),
    )
    .unwrap();
}
