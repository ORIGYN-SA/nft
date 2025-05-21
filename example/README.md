# Deploying an NFT Collection on Internet Computer

This guide will walk you through the process of deploying and managing an NFT collection on Internet Computer using the ICRC-7 standard.

## Prerequisites

- Internet Computer SDK (dfx) installed
- A funded identity with cycles (or identity who control a canister with cycles. Easiest way to do that is to go on https://nns.ic0.app/canisters/, login with II and then create a new canister with some ICP from your wallet. And then add as controller your local identity (via dfx, create a new identity, then use dfx identity get-principal to get the principal id to add). See https://internetcomputer.org/docs/building-apps/developer-tools/dfx/dfx-identity for more details)
- Basic understanding of Internet Computer concepts

## Step 1: Update canister_ids.json

Create or update your `canister_ids.json` file with your canister ID:

```json
{
  "nft": {
    "ic": "YOUR_CANISTER_ID"
  }
}
```

## Step 2: Check for Latest Version

Before deploying, check the latest version of the NFT canister at [ORIGYN NFT Releases](https://github.com/ORIGYN-SA/nft/releases). The current version used in this guide is v2025.05.21-db5cee0, but you should verify if a newer version is available, and update url in dfx.json according.

## Step 3: Deploy the Collection

Deploy your collection using the following command. Replace the principal ID with your own:

```bash
dfx deploy --network ic nft --mode reinstall --argument '(
  variant { Init = record {
    supply_cap = null;
    tx_window = null;
    test_mode = true;
    default_take_value = null;
    max_canister_storage_threshold = null;
    logo = null;
    permitted_drift = null;
    name = "MyCollection";
    minting_authorities = vec { principal "YOUR_PRINCIPAL_ID";};
    description = null;
    authorized_principals = vec { principal "YOUR_PRINCIPAL_ID";};
    version = record { major = 0 : nat32; minor = 0 : nat32; patch = 0 : nat32;};
    max_take_value = null;
    max_update_batch_size = null;
    max_query_batch_size = null;
    commit_hash = "commit_hash";
    max_memo_size = null;
    atomic_batch_transfers = null;
    collection_metadata = vec {};
    symbol = "MC";
    approval_init = record {
      max_approvals_per_token_or_collection = opt (10 : nat);
      max_revoke_approvals = opt (10 : nat);
    };
  }
})'
```

### Important Notes:
- The collection automatically manages storage canisters, so you don't need to create them manually
- Set a high storage threshold to ensure smooth operation
- Replace `YOUR_PRINCIPAL_ID` with your actual principal ID
- The `test_mode` parameter is set to `true` for testing purposes

## Step 4: Upload Files

1. First, compile the command-line tools:
```bash
cd ../cmdline
cargo build --release
```

2. Upload files using the compiled tool:
```bash
../target/release/origyn_icrc7_cmdlinetools upload-file \
  <CANISTER_ID> \
  <FILE_PATH> \
  <STORAGE_PATH> \
  <IDENTITY_FILE>
```

Example:
```bash
../target/release/origyn_icrc7_cmdlinetools upload-file \
  4sbzm-jaaaa-aaaaa-qah3a-cai \
  ~/Desktop/image.png \
  images/image.png \
  identity.pem
```

## Step 5: Minting NFTs

To mint an NFT, you need to be an authorized minting authority. Here's how to mint an NFT with metadata:

```bash
dfx canister call nft mint '(
  record {
    token_name = "My NFT";
    token_description = opt "Description of my NFT";
    token_logo = opt "https://example.com/logo.png";
    token_owner = record {
      owner = principal "YOUR_PRINCIPAL_ID";
      subaccount = null;
    };
    memo = null;
    token_metadata = opt vec {
      record { "key1"; variant { Text = "value1" } };
      record { "key2"; variant { Nat = 42 } };
    };
  }
)'
```

### Metadata Structure
The `token_metadata` field accepts a vector of key-value pairs where values can be of different types:
- Text: `variant { Text = "string value" }`
- Number: `variant { Nat = 42 }`
- Boolean: `variant { Bool = true }`
- Array: `variant { Array = vec { variant { Text = "item1" } } }`
- Map: `variant { Map = vec { record { "key"; variant { Text = "value" } } } }`

### Important Notes:
- Only authorized minting authorities can mint NFTs
- The token owner must be a valid principal ID
- Metadata is optional but recommended for better NFT discoverability
- The token name is required
- The token description and logo are optional. You can use the url you get from uploading file in the nft, or use any others url.

### Verifying Minted NFT
After minting, you can verify the NFT metadata using:

```bash
dfx canister call nft icrc7_token_metadata '(vec { 1 })'
```
This will return the metadata for the NFT with ID 1, including both the standard metadata (Name, Symbol) and any custom metadata you provided during minting.

and check who's the owner of the nft with :
```bash
dfx canister call nft icrc7_owner_of '(vec { 1 })'
```


## Additional Resources

- [Internet Computer Documentation](https://internetcomputer.org/docs/current/developer-docs/)
- [ICRC-7 Standard](https://github.com/dfinity/ICRC-7)