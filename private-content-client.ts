import { Principal } from "@dfinity/principal";
import {
  TransportSecretKey,
  DerivedPublicKey,
  EncryptedVetKey,
  VetKey,
  IbeCiphertext,
  IbeIdentity,
  IbeSeed,
} from "@dfinity/vetkeys";
import type { ActorSubclass, ActorMethod } from "@dfinity/agent";

// -------------------------
// Types
// -------------------------

export type NftId = string;

export interface EntryRegistration {
  name: string;
  readers: [] | [Principal[]]; // Option<Vec<Principal>>
}

export type PrivateContentStatus =
  | { PendingUpload: null }
  | { Active: null }
  | { PendingReencryption: null };

export interface EntryDetailResp {
  name: string;
  readers: [] | [Principal[]]; // Option<Vec<Principal>> — hidden from readers
  canonicalized_principals: Uint8Array | number[];
  status: [] | [PrivateContentStatus]; // None for "default_readers" pseudo-entry
  content_hash: [] | [Uint8Array | number[]]; // None for PendingUpload & default_readers
  content_size: [] | [bigint]; // None for entries without content & default_readers
}

export type PrivateContentError =
  | { NftNotFound: null }
  | { EntryNotFound: null }
  | { Unauthorized: null }
  | { ContentPendingReencryption: null }
  | { ContentNotAvailable: null }
  | { ContentHashRequired: null }
  | { InvalidContentHash: null }
  | { UploadNotInitialized: null }
  | { UploadAlreadyInProgress: null }
  | { UploadAlreadyFinalized: null }
  | { InvalidChunkData: null }
  | { IncompleteUpload: null }
  | { InvalidChunkSize: null }
  | { ContentTooLarge: null }
  | { TooManyReaders: null };

export type AccessRights =
  | { Read: null }
  | { ReadWrite: null }
  | { ReadWriteManage: null };

export interface ReaderInfo {
  rights: AccessRights;
  alias: [] | [string];
}

export type EncryptionMode = { SimpleAes: null };

export const DEFAULT_ENCRYPTION_MODE: EncryptionMode = { SimpleAes: null };

export type CandidResult<T> = { Ok: T } | { Err: PrivateContentError };

// -------------------------
// Constants
// -------------------------

export const MIN_CHUNK_SIZE = 1_000; // 1 KB
export const MAX_CHUNK_SIZE = 1_000_000; // 1 MB
export const MAX_UPLOAD_SIZE = 12_000_000; // 12 MB
export const MAX_READERS = 12;

// -------------------------
// Response types
// -------------------------

interface GetVetKeyPublicKeyResp {
  public_key: Uint8Array | number[];
}

interface GetVetKeyResp {
  encrypted_key: Uint8Array | number[];
}

interface SetReadersResp {
  canonical_principals: Uint8Array | number[];
  public_key: [] | [Uint8Array | number[]];
}

interface InitUploadResp {
  num_chunks: number;
}

interface GetNftAccessDetailResp {
  nft_access_detail: EntryDetailResp[];
}

interface DownloadPrivateContentResp {
  encrypted_content: Uint8Array | number[];
  content_hash: Uint8Array | number[];
  total_size: bigint;
}

// -------------------------
// Canister service interface
// -------------------------

export interface _SERVICE {
  // Updates (async inter-canister calls)
  get_vetkey_public_key: ActorMethod<
    [{ nft_id: NftId; entry_name: string }],
    CandidResult<GetVetKeyPublicKeyResp>
  >;
  get_vetkey: ActorMethod<
    [
      {
        transport_public_key: Uint8Array | number[];
        nft_id: NftId;
        entry_name: string;
      },
    ],
    CandidResult<GetVetKeyResp>
  >;
  set_readers: ActorMethod<
    [
      {
        nft_id: NftId;
        entry_name: [] | [string];
        readers: Principal[];
        need_pubkey: boolean;
      },
    ],
    CandidResult<SetReadersResp>
  >;
  init_private_upload: ActorMethod<
    [
      {
        hash: Uint8Array | number[];
        entry_name: string;
        default_readers: Array<[Principal, ReaderInfo]>;
        storage_canister_id: Principal;
        storage_path: string;
        plaintext_size: bigint;
        expected_chunks: number;
        chunk_size: [] | [bigint];
        total_size: bigint;
        encryption_mode: EncryptionMode;
      },
    ],
    CandidResult<InitUploadResp>
  >;
  store_private_chunk: ActorMethod<
    [
      {
        hash: Uint8Array | number[];
        entry_name: string;
        chunk_index: number;
        chunk_data: Uint8Array | number[];
      },
    ],
    CandidResult<{}>
  >;
  finalize_private_upload: ActorMethod<
    [
      {
        hash: Uint8Array | number[];
        entry_name: string;
      },
    ],
    CandidResult<{}>
  >;
  cancel_private_upload: ActorMethod<
    [
      {
        entry_name: string;
      },
    ],
    CandidResult<{}>
  >;

  // Queries
  get_my_nft_access_detail: ActorMethod<
    [{ nft_id: NftId }],
    CandidResult<GetNftAccessDetailResp>
  >;
  download_private_content: ActorMethod<
    [
      {
        nft_id: NftId;
        entry_name: string;
        offset: bigint;
        length: bigint;
      },
    ],
    CandidResult<DownloadPrivateContentResp>
  >;
}

/*
  // Usage example

  ```ts
  import { PrivateContentClient } from "./origyn-privacy-frontend-lib";

  const actor = createActor(storageCanisterId, { agentOptions: { identity } });
  const client = new PrivateContentClient(actor);

  // Get access details for a NFT (owner sees all entries + readers; reader sees own 
  // entries only)
  const details = await client.fetchNftAccessDetail(nftId);
  // Get the targeted entry by name
  const entry = await client.getEntry(details, "document1");

  // Owner uploads for the first time
  await client.encryptAndUpload(nftId, entry, fileBytes);

  // Reader downloads and decrypts (includes SHA-256 integrity verification)
  const content = await client.downloadAndDecrypt(nftId, entry);

  // Owner sets readers (replaces entire reader list)
  // Omit entryName for default_readers scope; pass true for need_pubkey optimization
  const newCanonicalPrincipals = await client.setReaders(
    nftId,
    [reader1Principal, reader2Principal],
    entry.name,
    true,
  );

  // After reader changes, owner re-encrypts
  await client.reencryptAndUpload(nftId, entry, newCanonicalPrincipals, fileBytes);
  ```
*/
export class PrivateContentClient {
  private cachedPublicKey?: DerivedPublicKey;

  constructor(private readonly actor: ActorSubclass<_SERVICE>) {}

  // Replace the underlying actor (e.g. after identity change).
  // The cached public key is preserved since it is canister-level and immutable.
  withActor(actor: ActorSubclass<_SERVICE>): PrivateContentClient {
    const client = new PrivateContentClient(actor);
    client.cachedPublicKey = this.cachedPublicKey;
    return client;
  }

  getEntry(details: EntryDetailResp[], entryName: string): EntryDetailResp {
    const entryDetail = details.find((d) => d.name === entryName);
    if (!entryDetail) throw new PrivateContentClientError("EntryNotFound");
    return entryDetail;
  }

  // --- Low-level canister calls -------------------------

  // The public key is canister-level and never changes. It is cached after
  // the first successful fetch. nftId/entryName are only needed for the
  // canister's authorization guard (caller must be owner or reader).
  async fetchPublicKey(
    nftId: NftId,
    entryName: string,
  ): Promise<DerivedPublicKey> {
    if (this.cachedPublicKey) return this.cachedPublicKey;
    const resp = unwrap<GetVetKeyPublicKeyResp>(
      await this.actor.get_vetkey_public_key({
        nft_id: nftId,
        entry_name: entryName,
      }),
    );
    const key = DerivedPublicKey.deserialize(toUint8Array(resp.public_key));
    this.cachedPublicKey = key;
    return key;
  }

  async fetchNftAccessDetail(nftId: NftId): Promise<EntryDetailResp[]> {
    const resp = unwrap<GetNftAccessDetailResp>(
      await this.actor.get_my_nft_access_detail({ nft_id: nftId }),
    );
    return resp.nft_access_detail;
  }

  async fetchVetKey(
    pubKey: DerivedPublicKey,
    nftId: NftId,
    entryName: string,
    canonicalizedPrincipals: Uint8Array,
  ): Promise<VetKey> {
    const tsk = TransportSecretKey.random();
    const resp = unwrap<GetVetKeyResp>(
      await this.actor.get_vetkey({
        transport_public_key: tsk.publicKeyBytes(),
        nft_id: nftId,
        entry_name: entryName,
      }),
    );
    const encrypted = EncryptedVetKey.deserialize(
      toUint8Array(resp.encrypted_key),
    );
    return encrypted.decryptAndVerify(tsk, pubKey, canonicalizedPrincipals);
  }

  // needPubkey: pass true when you intend to re-encrypt immediately after
  // changing readers. If the public key is not yet cached, it is fetched in
  // the same round-trip and cached for all subsequent calls. If the key is
  // already cached, need_pubkey is not sent to the canister — the cached
  // value is returned directly at no extra cost.
  async setReaders(
    nftId: NftId,
    readers: Principal[],
    entryName?: string,
    needPubkey = false,
  ): Promise<Uint8Array> {
    const resp = unwrap<SetReadersResp>(
      await this.actor.set_readers({
        nft_id: nftId,
        entry_name: entryName !== undefined ? [entryName] : [],
        readers,
        // Only ask the canister for the key when it is not already cached.
        need_pubkey: needPubkey && !this.cachedPublicKey,
      }),
    );
    if (resp.public_key[0]) {
      this.cachedPublicKey = DerivedPublicKey.deserialize(
        toUint8Array(resp.public_key[0]),
      );
    }
    return toUint8Array(resp.canonical_principals);
  }

  async initUpload(
    nftId: NftId,
    entryName: string,
    hash: Uint8Array,
    plaintextSize: bigint,
    totalSize: bigint,
    expectedChunks: number,
    storageCanisterId: Principal,
    storagePath: string,
    defaultReaders: Array<[Principal, ReaderInfo]> = [],
    chunkSize: bigint = BigInt(MAX_CHUNK_SIZE),
    encryptionMode: EncryptionMode = DEFAULT_ENCRYPTION_MODE,
  ): Promise<number> {
    const resp = unwrap<InitUploadResp>(
      await this.actor.init_private_upload({
        hash,
        entry_name: entryName,
        default_readers: defaultReaders,
        storage_canister_id: storageCanisterId,
        storage_path: storagePath,
        plaintext_size: plaintextSize,
        expected_chunks: expectedChunks,
        chunk_size: [chunkSize],
        total_size: totalSize,
        encryption_mode: encryptionMode,
      }),
    );
    return resp.num_chunks;
  }

  async storeChunk(
    hash: Uint8Array,
    entryName: string,
    chunkIndex: number,
    chunkData: Uint8Array,
  ): Promise<void> {
    unwrap<{}>(
      await this.actor.store_private_chunk({
        hash,
        entry_name: entryName,
        chunk_index: chunkIndex,
        chunk_data: chunkData,
      }),
    );
  }

  async finalizeUpload(hash: Uint8Array, entryName: string): Promise<void> {
    unwrap<{}>(
      await this.actor.finalize_private_upload({
        hash,
        entry_name: entryName,
      }),
    );
  }

  async cancelUpload(entryName: string): Promise<void> {
    unwrap<{}>(
      await this.actor.cancel_private_upload({
        entry_name: entryName,
      }),
    );
  }

  async downloadChunk(
    nftId: NftId,
    entryName: string,
    offset: bigint,
    length: bigint,
  ): Promise<{
    chunk: Uint8Array;
    contentHash: Uint8Array;
    totalSize: bigint;
  }> {
    const resp = unwrap<DownloadPrivateContentResp>(
      await this.actor.download_private_content({
        nft_id: nftId,
        entry_name: entryName,
        offset,
        length,
      }),
    );
    return {
      chunk: toUint8Array(resp.encrypted_content),
      contentHash: toUint8Array(resp.content_hash),
      totalSize: resp.total_size,
    };
  }

  // --- Crypto helpers (stateless) -------------------------

  static encrypt(
    content: Uint8Array,
    pubKey: DerivedPublicKey,
    canonicalizedPrincipals: Uint8Array,
  ): Uint8Array {
    const identity = IbeIdentity.fromBytes(canonicalizedPrincipals);
    const ciphertext = IbeCiphertext.encrypt(
      pubKey,
      identity,
      content,
      IbeSeed.random(),
    );
    return ciphertext.serialize();
  }

  static decrypt(encryptedContent: Uint8Array, vetKey: VetKey): Uint8Array {
    const ciphertext = IbeCiphertext.deserialize(encryptedContent);
    return ciphertext.decrypt(vetKey);
  }

  // --- High-level workflows -------------------------

  // Owner encrypts and uploads private content (handles chunked upload).
  // Computes SHA-256 of plaintext for the content integrity hash.
  async encryptAndUpload(
    nftId: NftId,
    entry: EntryDetailResp,
    content: Uint8Array,
    storageCanisterId: Principal,
    storagePath: string,
    defaultReaders: Array<[Principal, ReaderInfo]> = [],
    chunkSize = MAX_CHUNK_SIZE,
    encryptionMode: EncryptionMode = DEFAULT_ENCRYPTION_MODE,
  ): Promise<void> {
    const pubKey = await this.fetchPublicKey(nftId, entry.name);

    const canonicalPrincipals = toUint8Array(entry.canonicalized_principals);
    const encrypted = PrivateContentClient.encrypt(
      content,
      pubKey,
      canonicalPrincipals,
    );

    const contentHash = await sha256(content);
    await this.chunkedUpload(
      nftId,
      entry.name,
      encrypted,
      content,
      contentHash,
      storageCanisterId,
      storagePath,
      defaultReaders,
      chunkSize,
      encryptionMode,
    );
  }

  // Reader downloads and decrypts private content (handles chunked download).
  // Verifies SHA-256(decrypted) == content_hash from the canister.
  async downloadAndDecrypt(
    nftId: NftId,
    entry: EntryDetailResp,
  ): Promise<Uint8Array> {
    const canonicalPrincipals = toUint8Array(entry.canonicalized_principals);
    const chunkLength = BigInt(MAX_CHUNK_SIZE);

    // Download first chunk to discover total_size
    const first = await this.downloadChunk(nftId, entry.name, 0n, chunkLength);
    const { totalSize, contentHash } = first;

    const chunks: Uint8Array[] = [first.chunk];
    let offset = BigInt(first.chunk.length);

    while (offset < totalSize) {
      const next = await this.downloadChunk(
        nftId,
        entry.name,
        offset,
        chunkLength,
      );
      if (next.chunk.length === 0) break;
      chunks.push(next.chunk);
      offset += BigInt(next.chunk.length);
    }

    const encryptedContent = concatUint8Arrays(chunks);

    // Fetch public key and derive vetkey
    const pubKey = await this.fetchPublicKey(nftId, entry.name);
    const vetKey = await this.fetchVetKey(
      pubKey,
      nftId,
      entry.name,
      canonicalPrincipals,
    );

    const decrypted = PrivateContentClient.decrypt(encryptedContent, vetKey);

    // Verify integrity
    const computedHash = await sha256(decrypted);
    if (!uint8ArrayEquals(computedHash, contentHash)) {
      throw new PrivateContentClientError("ContentHashMismatch");
    }

    return decrypted;
  }

  // Owner re-encrypts after reader changes.
  // After calling setReaders(), entries become PendingReencryption.
  // Call this for each entry that needs re-encryption.
  async reencryptAndUpload(
    nftId: NftId,
    entry: EntryDetailResp,
    newCanonicalPrincipals: Uint8Array,
    content: Uint8Array,
    storageCanisterId: Principal,
    storagePath: string,
    defaultReaders: Array<[Principal, ReaderInfo]> = [],
    chunkSize = MAX_CHUNK_SIZE,
    encryptionMode: EncryptionMode = DEFAULT_ENCRYPTION_MODE,
  ): Promise<void> {
    const pubKey = await this.fetchPublicKey(nftId, entry.name);

    const encrypted = PrivateContentClient.encrypt(
      content,
      pubKey,
      newCanonicalPrincipals,
    );

    const contentHash = await sha256(content);
    await this.chunkedUpload(
      nftId,
      entry.name,
      encrypted,
      content,
      contentHash,
      storageCanisterId,
      storagePath,
      defaultReaders,
      chunkSize,
      encryptionMode,
    );
  }

  // --- Private helpers -------------------------

  private async chunkedUpload(
    nftId: NftId,
    entryName: string,
    encryptedContent: Uint8Array,
    plaintextContent: Uint8Array,
    contentHash: Uint8Array,
    storageCanisterId: Principal,
    storagePath: string,
    defaultReaders: Array<[Principal, ReaderInfo]> = [],
    chunkSize = MAX_CHUNK_SIZE,
    encryptionMode: EncryptionMode = DEFAULT_ENCRYPTION_MODE,
  ): Promise<void> {
    const totalSize = BigInt(encryptedContent.length);
    const plaintextSize = BigInt(plaintextContent.length);
    const numChunks = Number(
      Math.ceil(Number(totalSize) / Number(chunkSize)),
    );

    const numChunksFromCanister = await this.initUpload(
      nftId,
      entryName,
      contentHash,
      plaintextSize,
      totalSize,
      numChunks,
      storageCanisterId,
      storagePath,
      defaultReaders,
      BigInt(chunkSize),
      encryptionMode,
    );

    for (let i = 0; i < numChunksFromCanister; i++) {
      const start = i * chunkSize;
      const end = Math.min(start + chunkSize, encryptedContent.length);
      await this.storeChunk(
        contentHash,
        entryName,
        i,
        encryptedContent.slice(start, end),
      );
    }

    await this.finalizeUpload(contentHash, entryName);
  }
}

// -------------------------
// Convenience wrappers
// -------------------------

// Encrypt a UTF-8 string and upload.
export async function encryptTextAndUpload(
  client: PrivateContentClient,
  nftId: NftId,
  entry: EntryDetailResp,
  text: string,
): Promise<void> {
  return client.encryptAndUpload(nftId, entry, new TextEncoder().encode(text));
}

// Download and decrypt to a UTF-8 string.
export async function downloadAndDecryptText(
  client: PrivateContentClient,
  nftId: NftId,
  entry: EntryDetailResp,
): Promise<string> {
  const bytes = await client.downloadAndDecrypt(nftId, entry);
  return new TextDecoder().decode(bytes);
}

// -------------------------
// Helpers
// -------------------------

function unwrap<T>(result: CandidResult<T>): T {
  if ("Ok" in result) return result.Ok;
  const errKey = Object.keys(result.Err!)[0] as string;
  throw new PrivateContentClientError(errKey);
}

function toUint8Array(bytes: Uint8Array | number[]): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);
}

async function sha256(data: Uint8Array): Promise<Uint8Array> {
  const hashBuffer = await crypto.subtle.digest("SHA-256", data as Uint8Array<ArrayBuffer>);
  return new Uint8Array(hashBuffer);
}

function concatUint8Arrays(arrays: Uint8Array[]): Uint8Array {
  const totalLength = arrays.reduce((acc, arr) => acc + arr.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const arr of arrays) {
    result.set(arr, offset);
    offset += arr.length;
  }
  return result;
}

function uint8ArrayEquals(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

// Replicate the backend's `canonicalize_principals`:
// sort principal textual representations lexicographically, join with ","
export function canonicalizePrincipals(principals: Principal[]): Uint8Array {
  const sorted = principals.map((p) => p.toText()).sort();
  return new TextEncoder().encode(sorted.join(","));
}

export class PrivateContentClientError extends Error {
  constructor(public readonly code: string) {
    super(`PrivateContentError: ${code}`);
    this.name = "PrivateContentClientError";
  }
}
