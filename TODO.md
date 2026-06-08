## Direction (after team review)

The team's call:

1. **No AES layer.** Encrypt content directly with vetKey IBE. One ciphertext per entry, no per-entry symmetric key, no chunked AES-GCM format.
2. **Storage canister stays untouched.** No new endpoints, no new state, no awareness of NFTs.
3. **All NFT-aware logic lives in `core_nft`.** Owner, readers, entry registry, status, vetKey gateway, ownership transfer hooks.

---

## Why the branch's storage-side approach is wrong

The branch (`feature/privacy-with-vetkeys`) put all private content logic in the storage canister: NFT records, owners, readers, entry state machines, vetKey calls, ownership transfer hooks, \~2,500 lines of NFT-specific code. Six concrete problems with this placement:

**1. It breaks the existing separation.** For public content we already established a clean split: storage canister is a generic blob store keyed by `file_path` while `core_nft` is the brain that owns ownership, permissions, and upload routing. Putting private content logic into the storage canister inverts that pattern only for private content, which is confusing for anyone reading the code.

**2. Ownership lives in two places.** NFT ownership is authoritative in `core_nft` (ICRC7 ledger). The branch duplicates that into the storage canister so it can do owner-guarded operations there. Now every ICRC7 transfer has to fire a notification to the storage canister to keep ownership in sync. Two writes per transfer, two failure modes (transfer succeeds in `core_nft` but the storage canister sync fails or vice versa), and a permanent inconsistency risk.

**3. Storage canister sub-canisters don't scale.** We already shard public content across multiple storage sub-canisters (the existing `StorageSubCanisterManager`). If the storage canister also owns private content state, then private state has to be replicated or sharded across all those sub-canisters too. Or worse, all private content has to live in one sub-canister, which defeats the sharding model.

**4. The storage canister grows scope creep.** Today it does one thing: store bytes. Adding NFT-shaped state forces it into knowing about ICRC7 transfers, principals, identity construction, vetKey derivation. Future changes to NFT logic (new transfer types, new permission models) now require touching the storage canister too.

**5. Reusability of the storage canister.** Right now any team in the company could deploy a copy of the storage canister for any blob-storage need. The branch ties the storage canister to NFT-specific assumptions; it stops being a general-purpose component.

None of these problems are because Thomas's actual code is bad; the code on the branch is solid. The problem is **placement**. Moving the same logic into `core_nft` keeps every line that's worth keeping and removes the architectural issues.

---

## Why all the logic belongs in `core_nft`

Six reasons, mirroring the six problems above:

**1. Consistency with the public content pattern.** `core_nft` already owns the public upload facade. Putting private content next to it gives one location for everything an NFT canister knows about its NFTs.

**2. Single source of truth for ownership.** `core_nft` already has ownership in the ICRC7 ledger. Reading the current owner of NFT 42 is one lookup, never out of date.

**3. The storage canister can shard freely.** Multiple storage sub-canisters all behave identically (dumb byte stores). `core_nft` orchestrates which sub-canister holds which blob without needing the sub-canisters to know anything about NFTs.

**4. The storage canister stays a focused, small component.** Two files, maybe four endpoints, one job. Easier to audit, easier to upgrade, easier to reuse for other content needs.

---

## What we keep from the branch

Most of the actual work Thomas did is usable, it just lives in the wrong canister. What ports to `core_nft`:

* **The state machine** (`PendingUpload` → `Active` → `PendingReencryption`, with the same transition rules).
* **The reader inheritance logic** (`entry.readers == None` means inherit `default_readers`; `Some([])` means explicitly no readers).
* **Canonical principals construction** (sorted comma-joined principals as the IBE identity).
* **The `set_readers` semantics** (no-op detection when the effective set is unchanged, deduplication, owner stripping).
* **The integrity hash invariant** (immutable across re-encryption and ownership transfer).
* **The chunked upload bookkeeping** (`PendingUploadState`, chunk slots, size accumulation).
* **The integration test scenarios** (2,310 lines of edge case coverage, which port to the `origyn-nft` repo's PocketIC suite).

What gets dropped:

* The whole AES-GCM chunked body format we discussed (team rejected this).
* The Service Worker decryption pipeline for streaming (no longer needed; readers download whole ciphertext then decrypt).
* The custom `private_content_raw: StableBTreeMap` in the storage canister (the ciphertext just rides the existing public upload flow, addressed by `file_path` like any other blob).

---

## End-state architecture (one paragraph)

`core_nft` gains a new module that holds per-NFT private content state and a vetKey gateway. For each NFT, it stores a map of named entries; each entry tracks its readers, status, integrity hash, the storage canister URL where the encrypted blob lives, and the previous canonical identity (only during `PendingReencryption`, used by the owner to recover plaintext for re-encryption). Encrypted blobs are uploaded through the **existing public upload flow** and live on the storage canister exactly like any public file. Encryption is done in the browser using vetKey IBE addressed to the sorted-comma-joined principals; decryption requires asking `core_nft` for the vetKey, which `core_nft` derives via the IC management canister after checking the caller is authorized. The storage canister has zero changes from `master`.

---

## Implementation plan

Six work streams, roughly independent. Estimated effort in parentheses (rough order-of-magnitude, not committed estimates).

### 1: Delete branch's storage canister additions

No changes need to be made to storage canister, the code in the branch `master` works well for this scope of work.

### 2: Build the private content module on `core_nft`

In the `origyn-nft` repo, under `src/core_nft/src/`:

**New types module** (`types/private_content.rs`):

* `PrivateContentStatus` enum (PendingUpload, Active, PendingReencryption)
* `PrivateContentError` enum (port from branch, drop ones that no longer apply)
* `PrivateEntry` struct (status, readers, plaintext_hash, plaintext_size, canonical_identity, previous_canonical_identity, storage_canister_id, storage_path, pending_upload, format_version)
* `NftPrivateRecord` struct (default_readers, entries map)
* `EntryRegistration` (used in mint args)
* Helpers: canonical identity construction, reader inheritance lookup, equality comparison for no-op detection

**New state fields** on `Data`:

* `nft_private: BTreeMap<NftId, NftPrivateRecord>` (heap, `#[serde(default)]`)
* `vetkd_key_name: Option<String>` (`#[serde(default)]`)
* `vetkd_context: Option<String>` (`#[serde(default)]`)
* Constants: `MAX_PRIVATE_CONTENT_SIZE`, `MAX_READERS_PER_ENTRY`, chunk size limits

**New vetKey module** (`vetkeys.rs`, port from branch):

* `derive_public_key()` calling `vetkd_public_key`
* `derive_vetkey(identity, transport_pubkey)` calling `vetkd_derive_key`
* Both gated by "vetkd config has been set" check

**New guards** (additions to existing `guards.rs`):

* `caller_is_nft_owner(nft_id)`
* `caller_is_owner_or_any_reader(nft_id)`
* `caller_is_owner_or_entry_reader(nft_id, entry_name)`
* `entry_content_is_accessible(nft_id, entry_name)` (rejects PendingReencryption for readers, PendingUpload for everyone)

**New endpoints** (updates module):

* Extend the existing `mint` endpoint to accept a `private_content: Option<PrivateContentInit>` argument. On mint, register the entries with `status = Active` (since blobs are already uploaded).
* `set_readers(nft_id, entry_name: Option<String>, readers, need_pubkey) -> SetReadersResp { canonical_principals, public_key }`
* `init_private_content_reupload(nft_id, entry_name, plaintext_size, chunk_size) -> InitReuploadResp { num_chunks }`
* `store_private_content_chunk(nft_id, entry_name, chunk_index, chunk_data)`
* `commit_private_content_reupload(nft_id, entry_name, new_storage_path)`
* `get_vetkey(nft_id, entry_name, transport_public_key) -> GetVetkeyResp { encrypted_key }`
* `get_vetkey_for_reencryption(nft_id, entry_name, transport_public_key) -> GetVetkeyResp` (owner-only, only during PendingReencryption, derives for previous_canonical_identity)
* `set_vetkd_config(key_name, context)` (admin-only, one time only, gated by existing `manage_authorities` permission)

**New queries** (queries module):

* `get_my_nft_access_detail(nft_id) -> Vec<EntryDetailResp>` (owner sees all entries + a `default_readers` pseudo-entry; readers see only their entries with `readers: None`)
* `get_vetkey_public_key() -> GetVetkeyPublicKeyResp` (cached)

**ICRC7 transfer hook**:

* On every NFT ownership change, walk the entries map, clear `default_readers`, set every entry's `readers = None`, mark `Active` entries as `PendingReencryption`, set `previous_canonical_identity` to what was current pre-transfer, preserve ciphertext on storage. (The hook can live in the existing transfer code path inside `core_nft`.)

### Stream 3: Update mint to handle private entries

The mint endpoint signature changes:

```rust
struct MintArgs {
    metadata: ...,            // existing
    owner: Principal,         // existing
    private_content: Option<PrivateContentInit>,  // new
}

struct PrivateContentInit {
    default_readers: Vec<Principal>,
    entries: Vec<EntryInit>,
}

struct EntryInit {
    name: String,
    storage_canister_id: Principal,
    storage_path: String,
    plaintext_hash: [u8; 32],
    plaintext_size: u64,
    canonical_identity: Vec<u8>,
    readers: Option<Vec<Principal>>,
}
```

The mint flow becomes:

1. Validate the standard mint args (existing logic).
2. If `private_content` is provided:
   * Validate `vetkd_config` is set (else return `PrivateContentNotEnabled`).
   * For each entry, validate the storage canister + path is reachable (optionally, a quick get_storage_size call to confirm).
   * Build the `NftPrivateRecord`, set each entry to `Active`.
3. Mint the NFT (existing logic), then insert the private record under the new NFT id.
4. Return the new NFT id.

Pre-mint uploads use the existing `init_upload`/`store_chunk`/`finalize_upload` endpoints on `core_nft`. The minter picks an opaque `file_path` like `nft/pre-mint/{uuid}`. Optionally `core_nft` can later rewrite the path to `nft/{id}/private/{uuid}` on mint, but it's also fine to keep the pre-mint path.

### Stream 4: Rewrite the TypeScript client library

In a frontend repo (or temporarily at the repo root, like the branch):

**Encryption helpers**:

* `getMasterPublicKey(coreNftActor)` (cached)
* `computeCanonicalIdentity(owner, readers): Uint8Array`
* `ibeEncrypt(masterPub, identityBytes, plaintext): Uint8Array` (single-shot, no AES layer)
* `ibeDecrypt(vetkey, ciphertext): Uint8Array`

**Upload helper (pre-mint)**:

* `encryptAndUpload(coreNftActor, plaintext, ownerToBe, readers): Promise<EntryUploadResult>`
  * Generates canonical identity from `[owner, ...readers]`
  * IBE-encrypts plaintext for that identity
  * Uses the existing `init_upload`/`store_chunk`/`finalize_upload` on `core_nft` to push ciphertext to storage
  * Returns `{ storage_canister_id, storage_path, plaintext_hash, plaintext_size, canonical_identity }` ready for the mint call

**Mint helper**:

* `mintWithPrivateContent(coreNftActor, metadata, owner, defaultReaders, entries): Promise<NftId>`
  * Composes the `PrivateContentInit` from the uploaded entries
  * Calls `mint` once

**Download/decrypt helper**:

* `downloadAndDecrypt(coreNftActor, nftId, entryName): Promise<Uint8Array>`
  * Calls `get_my_nft_access_detail` to fetch entry metadata
  * Generates a transport keypair, calls `get_vetkey`
  * Unwraps the vetKey
  * HTTP-fetches the full ciphertext blob from the storage canister URL
  * IBE-decrypts using the vetKey
  * Verifies SHA-256 against `plaintext_hash`
  * Returns plaintext

**Reader rotation helper**:

* `setReadersAndReencrypt(coreNftActor, nftId, entryName, newReaders): Promise<void>`
  * Calls `set_readers` (with `need_pubkey: true`) to flip the entry to PendingReencryption
  * Calls `get_vetkey_for_reencryption` to derive the old vetKey
  * Downloads old ciphertext, decrypts with old vetKey
  * IBE-encrypts plaintext for new canonical identity
  * Uploads new ciphertext via the reupload endpoints
  * Calls `commit_private_content_reupload`

No Service Worker, no chunk math. The library is significantly simpler than the streaming version.

### Stream 5: Tests

Port the relevant scenarios from `integrations_tests/src/storage_suite/tests/test_private_content.rs` on the branch to the `origyn-nft` repo's PocketIC suite. Target test cases:

* Mint with private entries succeeds, entries are Active immediately.
* Mint with private entries fails when `vetkd_config` not set.
* Owner reads vetKey, decrypts content successfully.
* Reader reads vetKey, decrypts successfully.
* Anonymous principal rejected on all reader-protected endpoints.
* Reader rotation triggers PendingReencryption.
* Reader rotation no-op detection (same effective set) doesn't flip status.
* During PendingReencryption: reader gets `ContentPendingReencryption`, owner can call `get_vetkey_for_reencryption`.
* Re-upload completes the reencryption cycle, entry returns to Active, `previous_canonical_identity` cleared.
* ICRC7 transfer clears default_readers, sets each entry to PendingReencryption with previous_canonical_identity preserved.
* Integrity hash is immutable across re-encryption and ownership transfer.
* `get_my_nft_access_detail`: owner view shows all entries + default_readers pseudo-entry; reader view hides readers field and only shows accessible entries.
* Upload size limits respected.
* Set_readers rejects when reader list exceeds `MAX_READERS_PER_ENTRY`.
* Adding new entries post-mint is rejected (the rule from the design doc that slots are locked).

### Stream 6: Live deployment migration story

For existing deployed `core_nft` canisters in production:

* New state fields all have `#[serde(default)]`, so an upgrade with the new wasm deserializes existing state cleanly.
* After upgrade, every NFT exists without any private content (empty `nft_private` map).
* New admin endpoint `set_vetkd_config(key_name, context)` is callable by operators only once with the `manage_authorities` permission. Until called, private endpoints return `PrivateContentNotEnabled`.
* Storage canisters need no upgrade.

No data migration. Existing public flows are entirely untouched.

## Open questions

1. **Maximum entry count per NFT and Maximum size per entry.** Should we cap the number of private entries per NFT and the size as well? (Memory pressure on `core_nft` state.)
2. **Pre-mint upload garbage collection.** Same question as before: if someone uploads but never mints, we need a TTL-based cleanup. Suggestion: a background job in `core_nft` that walks unattached `nft/pre-mint/*` paths and deletes any older than 24 hours. Same problem exists for public uploads today.