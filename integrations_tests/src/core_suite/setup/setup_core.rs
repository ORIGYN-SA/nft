use crate::wasms::CORE_WASM;
use candid::encode_one;
use candid::Principal;
use pocket_ic::PocketIc;
use core_nft::lifecycle::Args;

pub fn setup_core_canister(
    pic: &mut PocketIc,
    buyback_burn_id: Principal,
    args: Args,
    controller: Principal
) -> Principal {
    let buyback_burn_wasm = CORE_WASM.clone();
    pic.add_cycles(buyback_burn_id, 1_000_000_000_000_000);

    pic.set_controllers(
        buyback_burn_id,
        Some(controller.clone()),
        vec![controller.clone()]
    ).unwrap();
    pic.tick();

    pic.install_canister(
        buyback_burn_id,
        buyback_burn_wasm,
        encode_one(args).unwrap(),
        Some(controller.clone())
    );

    buyback_burn_id
}
