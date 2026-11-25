//! Upgrading from the live wasm, whose ICRC3 block log runs a second memory
//! manager over the collection's stable memory, to one manager for everything.
//!
//! Each manager saved its own bucket count and memory sizes over the shared
//! header. The tests build those stale headers on the live wasm, upgrade, and
//! check that every block, token and approval is still there, that the header
//! is consistent again, and that memories then grow into new buckets without
//! touching each other's.

use crate::client::core_nft::{
    icrc37_approve_collection, icrc37_approve_tokens, icrc37_is_approved, icrc3_get_blocks,
    icrc7_token_metadata, mint, update_nft_metadata,
};
use crate::utils::{pin_clock_to_wall_time, random_principal, tick_n_blocks};
use crate::wasms::{CORE_WASM, CORE_WASM_LIVE};
use bity_ic_types::BuildVersion;
use candid::{encode_one, Nat, Principal};
use core_nft_api::lifecycle::init::{InitApprovalsArg, InitArgs};
use core_nft_api::lifecycle::Args;
use core_nft_api::post_upgrade::UpgradeArgs;
use core_nft_common::types::icrc37;
use core_nft_common::types::management::{mint as mint_types, update_nft_metadata as edit_types};
use core_nft_common::types::permissions::{Permission, PermissionManager};
use icrc_ledger_types::icrc::generic_value::ICRC3Value;
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc3::blocks::GetBlocksRequest;
use pocket_ic::{PocketIc, PocketIcBuilder};
use std::collections::HashMap;
use std::time::Duration;

const PAGE: usize = 65536;
const SIZES_AT: usize = 40;
const TABLE_AT: usize = SIZES_AT + 8 * 255;
const BUCKETS: usize = 32768;
const UNALLOCATED: u8 = 255;
/// About the size of a live certificate.
const SMALL: usize = 1_250;
/// Large enough that a few dozen tokens fill a bucket.
const LARGE: usize = 200_000;

/// The memory manager header and bucket table of a canister.
#[derive(Debug)]
struct Layout {
    allocated: usize,
    bucket_pages: u64,
    sizes: Vec<u64>,
    owners: Vec<u8>,
}

impl Layout {
    fn read(pic: &PocketIc, canister: Principal) -> Self {
        let stable = pic.get_stable_memory(canister);
        assert_eq!(&stable[0..3], b"MGR", "no memory manager header");
        Self {
            allocated: u16::from_le_bytes([stable[4], stable[5]]) as usize,
            bucket_pages: u16::from_le_bytes([stable[6], stable[7]]) as u64,
            sizes: (0..255)
                .map(|id| {
                    let at = SIZES_AT + 8 * id;
                    u64::from_le_bytes(stable[at..at + 8].try_into().unwrap())
                })
                .collect(),
            owners: stable[TABLE_AT..TABLE_AT + BUCKETS].to_vec(),
        }
        .checked_against_physical_size(stable.len())
    }

    fn checked_against_physical_size(self, stable_len: usize) -> Self {
        let physical = (stable_len / PAGE - 1) as u64 / self.bucket_pages;
        for (bucket, owner) in self.owners.iter().enumerate() {
            if *owner != UNALLOCATED {
                assert!((bucket as u64) < physical, "bucket {bucket} lies past the end");
            }
        }
        self
    }

    fn buckets_of(&self, id: u8) -> Vec<usize> {
        (0..BUCKETS).filter(|bucket| self.owners[*bucket] == id).collect()
    }

    /// Problems a single manager never leaves: a count that would hand out an
    /// owned bucket again, or a memory whose size does not match its buckets.
    fn problems(&self) -> Vec<String> {
        let mut problems = vec![];
        let highest = self.owners.iter().rposition(|owner| *owner != UNALLOCATED);
        let wanted = highest.map_or(0, |bucket| bucket + 1);
        if self.allocated != wanted {
            problems.push(format!("{} buckets allocated, {wanted} in use", self.allocated));
        }
        for id in 0..255u8 {
            let needed = self.sizes[id as usize].div_ceil(self.bucket_pages) as usize;
            let owned = self.buckets_of(id).len();
            if needed != owned {
                problems.push(format!(
                    "memory {id} records {} pages and owns {owned} buckets",
                    self.sizes[id as usize]
                ));
            }
        }
        problems
    }
}

struct Collection {
    pic: PocketIc,
    controller: Principal,
    owner: Principal,
    canister: Principal,
}

fn init_args(controller: Principal) -> InitArgs {
    let mut permissions = HashMap::new();
    permissions.insert(
        controller,
        vec![
            Permission::Minting,
            Permission::ManageAuthorities,
            Permission::UpdateMetadata,
            Permission::UpdateCollectionMetadata,
            Permission::ReadUploads,
            Permission::UpdateUploads,
        ],
    );
    InitArgs {
        test_mode: false,
        version: BuildVersion::min(),
        commit_hash: "commit_hash".to_string(),
        permissions: PermissionManager::new(permissions),
        description: Some("Claimlink style collection".to_string()),
        symbol: "CL".to_string(),
        name: "ClaimlinkStyle".to_string(),
        logo: None,
        supply_cap: None,
        max_query_batch_size: None,
        max_update_batch_size: None,
        max_take_value: None,
        default_take_value: None,
        max_memo_size: None,
        atomic_batch_transfers: None,
        tx_window: None,
        permitted_drift: None,
        max_canister_storage_threshold: None,
        collection_metadata: HashMap::new(),
        approval_init: InitApprovalsArg {
            max_approvals_per_token_or_collection: Some(Nat::from(100u64)),
            max_revoke_approvals: Some(Nat::from(100u64)),
        },
        base_url: None,
        vetkd_key_name: "dfx_test_key".to_string(),
        vetkd_context: "vetkd_context".to_string(),
        storage_cycles: None,
    }
}

/// A claimlink-style collection on the live wasm.
fn install_live() -> Collection {
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    pin_clock_to_wall_time(&pic);
    let controller = random_principal();
    let canister = pic.create_canister_with_settings(Some(controller), None);
    pic.add_cycles(canister, 100_000_000_000_000_000_000);
    pic.install_canister(
        canister,
        CORE_WASM_LIVE.clone(),
        encode_one(Args::Init(init_args(controller))).unwrap(),
        Some(controller),
    );
    pic.advance_time(Duration::from_secs(60));
    tick_n_blocks(&pic, 5);
    Collection {
        pic,
        controller,
        owner: random_principal(),
        canister,
    }
}

impl Collection {
    fn upgrade_to(&mut self, wasm: Vec<u8>) {
        // install_code is rate limited per canister.
        tick_n_blocks(&self.pic, 100);
        self.pic
            .upgrade_canister(
                self.canister,
                wasm,
                encode_one(Args::Upgrade(UpgradeArgs {
                    version: BuildVersion::min(),
                    commit_hash: "commit_hash 2".to_string(),
                    vetkd_key_name: None,
                    vetkd_context: None,
                    base_url: None,
                    storage_cycles: None,
                }))
                .unwrap(),
                Some(self.controller),
            )
            .expect("upgrade");
        self.pic.advance_time(Duration::from_secs(1));
        tick_n_blocks(&self.pic, 10);
    }

    fn mint(&mut self, from: u64, count: u64, size: usize) -> Vec<Nat> {
        (from..from + count)
            .map(|n| {
                let token_id = mint(
                    &mut self.pic,
                    self.controller,
                    self.canister,
                    &mint_types::Args {
                        mint_requests: vec![mint_types::MintRequest {
                            token_owner: Account {
                                owner: self.owner,
                                subaccount: None,
                            },
                            memo: None,
                            metadata: json_data(certificate(n, 1, size)),
                            private_content: None,
                            public_content: None,
                        }],
                    },
                )
                .expect("mint");
                self.pic.advance_time(Duration::from_secs(1));
                self.pic.tick();
                token_id
            })
            .collect()
    }

    /// Rewrites `token_ids`, the tokens minted as numbers `first` onwards.
    fn edit(&mut self, token_ids: &[Nat], first: u64, version: u64, size: usize) {
        for (n, token_id) in (first..).zip(token_ids) {
            update_nft_metadata(
                &mut self.pic,
                self.controller,
                self.canister,
                &edit_types::Args {
                    token_id: token_id.clone(),
                    metadata: json_data(certificate(n, version, size)),
                    memo: None,
                },
            )
            .expect("edit");
            self.pic.advance_time(Duration::from_secs(1));
            self.pic.tick();
        }
    }

    fn approve_token(&mut self, token_id: &Nat, spender: Principal) {
        let approval_info = self.approval_info(spender);
        let result = icrc37_approve_tokens(
            &mut self.pic,
            self.owner,
            self.canister,
            &vec![icrc37::icrc37_approve_tokens::ApproveTokenArg {
                token_id: token_id.clone(),
                approval_info,
            }],
        )
        .expect("approve the token");
        assert!(
            matches!(result[0], Some(icrc37::icrc37_approve_tokens::ApproveTokenResult::Ok(_))),
            "token approval refused: {result:?}"
        );
        self.pic.advance_time(Duration::from_secs(1));
        self.pic.tick();
    }

    fn approve_collection(&mut self, spender: Principal) {
        let approval_info = self.approval_info(spender);
        let result = icrc37_approve_collection(
            &mut self.pic,
            self.owner,
            self.canister,
            &vec![icrc37::icrc37_approve_collection::ApproveCollectionArg { approval_info }],
        )
        .expect("approve the collection");
        assert!(
            matches!(
                result[0],
                Some(icrc37::icrc37_approve_collection::ApproveCollectionResult::Ok(_))
            ),
            "collection approval refused: {result:?}"
        );
        self.pic.advance_time(Duration::from_secs(1));
        self.pic.tick();
    }

    fn approval_info(&self, spender: Principal) -> icrc37::ApprovalInfo {
        icrc37::ApprovalInfo {
            spender: Account {
                owner: spender,
                subaccount: None,
            },
            from_subaccount: None,
            expires_at: None,
            memo: None,
            created_at_time: self.pic.get_time().as_nanos_since_unix_epoch(),
        }
    }

    fn is_approved(&self, spender: Principal, token_id: &Nat) -> bool {
        icrc37_is_approved(
            &self.pic,
            self.controller,
            self.canister,
            &vec![icrc37::icrc37_is_approved::IsApprovedArg {
                spender: Account {
                    owner: spender,
                    subaccount: None,
                },
                from_subaccount: None,
                token_id: token_id.clone(),
            }],
        )[0]
    }

    /// The spenders this collection's owner approved for all its tokens.
    fn collection_spenders(&self) -> Vec<Principal> {
        let owner = Account {
            owner: self.owner,
            subaccount: None,
        };
        let arg = candid::encode_args((
            owner,
            None::<icrc37::icrc37_get_collection_approvals::CollectionApproval>,
            Some(Nat::from(10u64)),
        ))
        .unwrap();
        let reply = self
            .pic
            .query_call(
                self.canister,
                self.controller,
                "icrc37_get_collection_approvals",
                arg,
            )
            .expect("icrc37_get_collection_approvals");
        candid::decode_one::<icrc37::icrc37_get_collection_approvals::Response>(&reply)
            .unwrap()
            .into_iter()
            .map(|approval| approval.approval_info.spender.owner)
            .collect()
    }

    fn layout(&self) -> Layout {
        Layout::read(&self.pic, self.canister)
    }

    /// Every block and every token's metadata, read in pages of `page`.
    fn snapshot(&self, token_ids: &[Nat], page: u64) -> (Vec<String>, Vec<String>) {
        let length = icrc3_get_blocks(
            &self.pic,
            self.controller,
            self.canister,
            &vec![GetBlocksRequest {
                start: Nat::from(0u64),
                length: Nat::from(0u64),
            }],
        )
        .log_length;
        let length: u64 = length.0.try_into().unwrap();
        let mut blocks = vec![];
        while (blocks.len() as u64) < length {
            let result = icrc3_get_blocks(
                &self.pic,
                self.controller,
                self.canister,
                &vec![GetBlocksRequest {
                    start: Nat::from(blocks.len() as u64),
                    length: Nat::from(page),
                }],
            );
            assert!(!result.blocks.is_empty(), "no blocks from {}", blocks.len());
            blocks.extend(result.blocks.iter().map(|block| format!("{:?}", block)));
        }
        let mut metadata = vec![];
        for chunk in token_ids.chunks(page as usize) {
            for entry in icrc7_token_metadata(&self.pic, self.controller, self.canister, &chunk.to_vec()) {
                metadata.push(format!("{:?}", entry.expect("the token has metadata")));
            }
        }
        (blocks, metadata)
    }

    /// Token number `n` holds the certificate last written to it, whose
    /// version and size `expected(n)` gives.
    fn assert_metadata(&self, token_ids: &[Nat], expected: impl Fn(u64) -> (u64, usize)) {
        for (chunk_start, chunk) in (0u64..).step_by(4).zip(token_ids.chunks(4)) {
            let entries = icrc7_token_metadata(&self.pic, self.controller, self.canister, &chunk.to_vec());
            for ((n, token_id), entry) in (chunk_start..).zip(chunk).zip(entries) {
                let (version, size) = expected(n);
                let json = entry
                    .expect("the token has metadata")
                    .into_iter()
                    .find(|(key, _)| key == "JSON_DATA")
                    .map(|(_, value)| value);
                assert_eq!(
                    json,
                    Some(ICRC3Value::Text(certificate(n, version, size))),
                    "token {token_id}"
                );
            }
        }
    }
}

fn json_data(text: String) -> Vec<(String, ICRC3Value)> {
    vec![("JSON_DATA".to_string(), ICRC3Value::Text(text))]
}

fn certificate(n: u64, version: u64, size: usize) -> String {
    format!(
        r#"{{"title":"Certificate {n}","version":{version},"body":"{}"}}"#,
        "x".repeat(size)
    )
}

#[test]
fn test_a_never_upgraded_live_collection_keeps_everything() {
    let mut collection = install_live();
    let token_ids = collection.mint(0, 40, SMALL);
    collection.edit(&token_ids, 0, 2, SMALL);
    let stale = collection.layout().problems();
    assert!(!stale.is_empty(), "the live wasm left a consistent header");
    let before = collection.snapshot(&token_ids, 50);

    collection.upgrade_to(CORE_WASM.clone());

    assert_eq!(collection.layout().problems(), Vec::<String>::new());
    assert_eq!(collection.snapshot(&token_ids, 50), before);
    collection.assert_metadata(&token_ids, |_| (2, SMALL));
}

#[test]
fn test_an_approval_made_on_the_live_wasm_survives_the_upgrade() {
    let token_spender = random_principal();
    let collection_spender = random_principal();
    let mut collection = install_live();
    let token_ids = collection.mint(0, 10, SMALL);
    collection.upgrade_to(CORE_WASM_LIVE.clone());
    collection.approve_token(&token_ids[0], token_spender);
    // The block log's manager saves its stale header as the log grows.
    for version in 2..5 {
        collection.edit(&token_ids, 0, version, SMALL);
    }
    let stale = collection.layout().problems();
    assert!(!stale.is_empty(), "the live wasm left a consistent header");
    let before = collection.snapshot(&token_ids, 50);

    collection.upgrade_to(CORE_WASM.clone());

    assert_eq!(collection.layout().problems(), Vec::<String>::new());
    assert_eq!(collection.snapshot(&token_ids, 50), before);
    assert!(collection.is_approved(token_spender, &token_ids[0]));

    collection.approve_collection(collection_spender);
    let layout = collection.layout();
    assert_eq!(layout.problems(), Vec::<String>::new());
    let token_approval_buckets = layout.buckets_of(3);
    let collection_approval_buckets = layout.buckets_of(4);
    assert!(!token_approval_buckets.is_empty() && !collection_approval_buckets.is_empty());
    assert!(token_approval_buckets
        .iter()
        .all(|bucket| !collection_approval_buckets.contains(bucket)));
    assert!(collection.is_approved(token_spender, &token_ids[0]));
    assert_eq!(collection.collection_spenders(), vec![collection_spender]);

    collection.upgrade_to(CORE_WASM.clone());

    assert_eq!(collection.layout().problems(), Vec::<String>::new());
    assert!(collection.is_approved(token_spender, &token_ids[0]));
    assert_eq!(collection.collection_spenders(), vec![collection_spender]);
}

#[test]
fn test_memories_grow_into_new_buckets_without_overlap_after_the_upgrade() {
    let mut collection = install_live();
    let mut token_ids = collection.mint(0, 10, SMALL);
    collection.edit(&token_ids, 0, 2, SMALL);
    collection.upgrade_to(CORE_WASM.clone());

    // Metadata passes one bucket, then the block log does, with approvals
    // claiming new buckets in between.
    token_ids.extend(collection.mint(10, 45, LARGE));
    collection.approve_token(&token_ids[0], random_principal());
    let large = token_ids[10..35].to_vec();
    collection.edit(&large, 10, 2, LARGE);
    collection.approve_collection(random_principal());

    let layout = collection.layout();
    assert_eq!(layout.problems(), Vec::<String>::new());
    assert!(layout.buckets_of(5).len() >= 2, "metadata stayed in one bucket: {layout:?}");
    assert!(layout.buckets_of(1).len() >= 2, "the block log stayed in one bucket: {layout:?}");
    let version_of = |n: u64| match n {
        n if n < 10 => (2, SMALL),
        n if n < 35 => (2, LARGE),
        _ => (1, LARGE),
    };
    collection.assert_metadata(&token_ids, version_of);
    let before = collection.snapshot(&token_ids, 2);
    assert_eq!(before.0.len(), 10 + 10 + 45 + 1 + 25 + 1);

    collection.upgrade_to(CORE_WASM.clone());

    assert_eq!(collection.layout().problems(), Vec::<String>::new());
    assert_eq!(collection.snapshot(&token_ids, 2), before);
    collection.assert_metadata(&token_ids, version_of);
}
