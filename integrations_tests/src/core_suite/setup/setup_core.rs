use crate::wasms::CORE_WASM;
use candid::encode_one;
use candid::types::value::IDLValue;
use candid::Principal;
use core_nft::lifecycle::Args;
use pocket_ic::PocketIc;

pub fn setup_core_canister(
    pic: &mut PocketIc,
    core_canister_id: Principal,
    args: Args,
    controller: Principal,
) -> Principal {
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
