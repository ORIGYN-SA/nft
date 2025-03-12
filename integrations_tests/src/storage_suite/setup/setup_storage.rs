use crate::wasms::STORAGE_WASM;
use candid::encode_one;
use candid::Principal;
use pocket_ic::PocketIc;
use storage_api_canister::lifecycle::Args;

pub fn setup_storage_canister(
    pic: &mut PocketIc,
    storage_canister_id: Principal,
    args: Args,
    controller: Principal,
) -> Principal {
    let core_nft_wasm = STORAGE_WASM.clone();
    pic.add_cycles(storage_canister_id, 100_000_000_000_000_000);

    pic.set_controllers(
        storage_canister_id,
        Some(controller.clone()),
        vec![controller.clone()],
    )
    .unwrap();
    pic.tick();

    pic.install_canister(
        storage_canister_id,
        core_nft_wasm,
        encode_one(args).unwrap(),
        Some(controller.clone()),
    );

    storage_canister_id
}

pub fn upgrade_storage_canister(
    pic: &mut PocketIc,
    storage_canister_id: Principal,
    args: Args,
    controller: Principal,
) {
    let core_nft_wasm = STORAGE_WASM.clone();
    pic.add_cycles(storage_canister_id, 100_000_000_000_000_000);

    pic.set_controllers(
        storage_canister_id,
        Some(controller.clone()),
        vec![controller.clone()],
    )
    .unwrap();
    pic.tick();

    pic.upgrade_canister(
        storage_canister_id,
        core_nft_wasm,
        encode_one(args).unwrap(),
        Some(controller.clone()),
    )
    .unwrap();
}
