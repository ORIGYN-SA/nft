use crate::wasms::{CORE_WASM, REGISTRY_WASM};

use candid::encode_one;
use candid::types::value::IDLValue;
use candid::Principal;
use core_nft::lifecycle::Args;
use pocket_ic::PocketIc;

#[derive(Clone, Debug, Default, candid::CandidType, candid::Deserialize)]
pub struct RegistryCanisterInitPayload {
    pub mutations: Vec<u64>,
    // pub mutations: Vec<RegistryAtomicMutateRequest>, this is the real type, but we don't need it for now as we init with empty vec
}

pub fn setup_core_canister(
    pic: &mut PocketIc,
    core_canister_id: Principal,
    registry_canister_id: Principal,
    args: Args,
    controller: Principal,
) -> Principal {
    pic.add_cycles(registry_canister_id, 100_000_000_000_000_000_000);

    pic.set_controllers(
        registry_canister_id,
        Some(controller.clone()),
        vec![controller.clone()],
    )
    .unwrap();

    pic.tick();

    pic.install_canister(
        registry_canister_id,
        REGISTRY_WASM.clone(),
        encode_one(&RegistryCanisterInitPayload { mutations: vec![] }).unwrap(),
        Some(controller.clone()),
    );

    pic.tick();

    let core_nft_wasm = CORE_WASM.clone();
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
        core_nft_wasm,
        encode_one(args).unwrap(),
        Some(controller.clone()),
    );

    core_canister_id
}
