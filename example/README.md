# Deploying an NFT Collection on Internet Computer

This guide will walk you through the process of deploying and managing an NFT collection on Internet Computer using the ICRC-7 standard.

## Environment Variables Setup

To make it easier to use the commands, you can set up the following environment variables in your shell:

```bash
# Canister IDs
export NFT_CANISTER_ID="YOUR_CANISTER_ID"

# Principals
export YOUR_PRINCIPAL_ID="YOUR_PRINCIPAL_ID"

# Configuration
export COLLECTION_NAME="MyCollection"
export COLLECTION_SYMBOL="MC"
export IDENTITY_FILE="$(dfx identity whoami).pem"
```

## Prerequisites

- Internet Computer SDK (dfx) installed
- A funded identity with cycles (or identity who control a canister with cycles. Easiest way to do that is to go on https://nns.ic0.app/canisters/, login with II and then create a new canister with some ICP from your wallet. And then add as controller your local identity (via dfx, create a new identity, then use dfx identity get-principal to get the principal id to add). See https://internetcomputer.org/docs/building-apps/developer-tools/dfx/dfx-identity for more details)
- Basic understanding of Internet Computer concepts

## Step 1: Update canister_ids.json

The deployment script automatically updates your `canister_ids.json` file with the correct canister ID. However, if you need to do it manually, you can use these commands:

```bash
sed -i '' "s/YOUR_CANISTER_ID/$NFT_CANISTER_ID/g" canister_ids.json
```

Or if you're on Linux:
```bash
sed -i "s/YOUR_CANISTER_ID/$NFT_CANISTER_ID/g" canister_ids.json
```

The file should look like this:
```json
{
  "nft": {
    "ic": "$NFT_CANISTER_ID"
  }
}
```

## Step 2: Check for Latest Version

Before deploying, check the latest version of the NFT canister at [ORIGYN NFT Releases](https://github.com/ORIGYN-SA/nft/releases). The current version used in this guide is v2025.05.21-db5cee0, but you should verify if a newer version is available, and update url in dfx.json accordingly.

**Note**: The deployment script uses version 1.1.1 for production compatibility.

## Step 3: Deploy the Collection

You have two options for deploying your collection:

### Option A: Automated Deployment (Recommended)

Use the automated deployment script that handles everything for you:

```bash
./deploy_collection.sh
```

This script will:
- Check prerequisites
- Set up environment variables interactively
- Create the identity file automatically
- Update canister_ids.json
- Deploy the collection
- Build the CLI tool
- Test file upload
- Optionally mint NFTs

### Option B: Manual Deployment

If you prefer to deploy manually, use the following command:

```bash
dfx deploy --network ic nft --mode reinstall --argument '(
  variant {
    Init = record {
      permissions = record {
        user_permissions = vec {
          record {
            principal "$YOUR_PRINCIPAL_ID";
            vec {
              variant { UpdateMetadata };
              variant { Minting };
              variant { UpdateCollectionMetadata };
              variant { UpdateUploads };
              variant { ManageAuthorities };
              variant { ReadUploads };
            };
          };
        };
      };
      supply_cap = null;
      tx_window = null;
      test_mode = false;
      default_take_value = null;
      max_canister_storage_threshold = null;
      logo = null;
      permitted_drift = null;
      name = "$COLLECTION_NAME";
      description = opt "$COLLECTION_DESCRIPTION";
      version = record {
        major = 1 : nat32;
        minor = 1 : nat32;
        patch = 1 : nat32;
      };
      max_take_value = null;
      max_update_batch_size = null;
      max_query_batch_size = null;
      commit_hash = "aaa";
      max_memo_size = null;
      atomic_batch_transfers = null;
      collection_metadata = vec {};
      symbol = "$COLLECTION_SYMBOL";
      approval_init = record {
        max_approvals_per_token_or_collection = null;
        max_revoke_approvals = null;
      };
    }
  }
)'
```

### Important Notes:
- The collection automatically manages storage canisters, so you don't need to create them manually
- Set a high storage threshold to ensure smooth operation
- Replace `YOUR_PRINCIPAL_ID` with your actual principal ID
- The `test_mode` parameter is set to `false` for production use
- The `permissions` field uses the new structure with `user_permissions` and specific permission variants
- All permissions are granted to your principal for full control of the collection
- The version is set to 1.1.1 for production compatibility

## Step 4: Build the CLI Tool

**If you used the automated script**: The CLI tool was built automatically.

**If you deployed manually**: Compile the ICRC7 NFT command-line tool:

```bash
cd ../cmdline
cargo build --release
```

The tool will be available at `../target/release/origyn_icrc7_cmdlinetools`.

## Step 5: Upload Files to the Collection

**If you used the automated script**: The identity file was created automatically and is ready to use.

**If you deployed manually**: Export your identity to a .pem file first:
```bash
dfx identity export $(dfx identity whoami) > $(dfx identity whoami).pem
export IDENTITY_FILE="$(dfx identity whoami).pem"
```

Then upload your files:
```bash
../target/release/origyn_icrc7_cmdlinetools \
  --network ic \
  --identity $IDENTITY_FILE \
  --canister $NFT_CANISTER_ID \
  upload-file ./origynlogo.png origynlogo.png
```

### Upload Options:
- `--chunk_size`: Specify chunk size in bytes (default: 1MB)
- The tool shows upload progress with a progress bar

## Step 6: Create and Validate Metadata

### Interactive Metadata Creation

Create ICRC97-compliant metadata interactively:

```bash
../target/release/origyn_icrc7_cmdlinetools \
  --network ic \
  --identity $IDENTITY_FILE \
  --canister $NFT_CANISTER_ID \
  create-metadata --output metadata.json --interactive
```

### CLI Metadata Creation

Create metadata using command-line parameters:

```bash
../target/release/origyn_icrc7_cmdlinetools \
  --network ic \
  --identity $IDENTITY_FILE \
  --canister $NFT_CANISTER_ID \
  create-metadata \
  --output metadata.json \
  --name "My NFT" \
  --description "A beautiful NFT" \
  --image "https://$NFT_CANISTER_ID.raw.icp0.io/images/origynlogo.png" \
  --attribute "Rarity:Legendary:boost_number" \
  --attribute "Power:95:number" \
  --attribute "Element:Fire"
```

### Validate Existing Metadata

Validate an existing JSON metadata file:

```bash
../target/release/origyn_icrc7_cmdlinetools \
  --network ic \
  --identity $IDENTITY_FILE \
  --canister $NFT_CANISTER_ID \
  validate-metadata metadata.json
```

## Step 7: Upload Metadata

Upload your metadata file to the collection:

```bash
../target/release/origyn_icrc7_cmdlinetools \
  --network ic \
  --identity $IDENTITY_FILE \
  --canister $NFT_CANISTER_ID \
  upload-metadata metadata.json
```

This will return a metadata URL that you can use for minting.

## Step 8: Mint NFTs

### Method 1: Mint with ICRC97 URL

```bash
../target/release/origyn_icrc7_cmdlinetools \
  --network ic \
  --identity $IDENTITY_FILE \
  --canister $NFT_CANISTER_ID \
  mint \
  --owner $YOUR_PRINCIPAL_ID \
  --name "My NFT" \
  --icrc97_url "https://$NFT_CANISTER_ID.raw.icp0.io/abc123.json" \
  --memo "First NFT"
```

### Method 2: Mint with Interactive Metadata Creation

```bash
../target/release/origyn_icrc7_cmdlinetools \
  --network ic \
  --identity $IDENTITY_FILE \
  --canister $NFT_CANISTER_ID \
  mint \
  --owner $YOUR_PRINCIPAL_ID \
  --name "My Interactive NFT" \
  --interactive
```

### Method 3: Mint with Direct Metadata Entries

```bash
../target/release/origyn_icrc7_cmdlinetools \
  --network ic \
  --identity $IDENTITY_FILE \
  --canister $NFT_CANISTER_ID \
  mint \
  --owner $YOUR_PRINCIPAL_ID \
  --name "My CLI NFT" \
  --metadata "description:A beautiful NFT created via CLI" \
  --metadata "image:https://$NFT_CANISTER_ID.raw.icp0.io/images/origynlogo.png" \
  --metadata "rarity:Legendary" \
  --metadata "power:95" \
  --memo "CLI created NFT"
```

## Step 9: Manage Permissions

The CLI tool also provides commands to manage permissions on your NFT collection:

### Grant Permissions
```bash
../target/release/origyn_icrc7_cmdlinetools \
  --network ic \
  --identity $IDENTITY_FILE \
  --canister $NFT_CANISTER_ID \
  permissions grant \
  --principal "YOUR_TARGET_PRINCIPAL" \
  --permission "minting"
```

### Revoke Permissions
```bash
../target/release/origyn_icrc7_cmdlinetools \
  --network ic \
  --identity $IDENTITY_FILE \
  --canister $NFT_CANISTER_ID \
  permissions revoke \
  --principal "YOUR_TARGET_PRINCIPAL" \
  --permission "minting"
```

### List Permissions
```bash
../target/release/origyn_icrc7_cmdlinetools \
  --network ic \
  --identity $IDENTITY_FILE \
  --canister $NFT_CANISTER_ID \
  permissions list \
  --principal "YOUR_TARGET_PRINCIPAL"
```

### Check Permission
```bash
../target/release/origyn_icrc7_cmdlinetools \
  --network ic \
  --identity $IDENTITY_FILE \
  --canister $NFT_CANISTER_ID \
  permissions has \
  --principal "YOUR_TARGET_PRINCIPAL" \
  --permission "minting"
```

### Available Permissions:
- `minting`: Can mint new NFTs
- `manage_authorities`: Can manage other authorities
- `update_metadata`: Can update token metadata
- `update_collection_metadata`: Can update collection metadata
- `read_uploads`: Can read uploaded files
- `update_uploads`: Can upload new files

## ICRC97 Metadata Format

The tool creates and validates metadata according to the ICRC97 standard:

```json
{
  "name": "NFT Name",
  "description": "NFT Description",
  "image": "https://example.com/image.png",
  "external_url": "https://example.com",
  "attributes": [
    {
      "trait_type": "Rarity",
      "value": "Legendary",
      "display_type": "boost_number"
    },
    {
      "trait_type": "Power",
      "value": 95,
      "display_type": "number"
    }
  ]
}
```

### Supported Display Types:
- `number`: Regular number display
- `boost_number`: Number with + prefix
- `boost_percentage`: Percentage with + prefix  
- `date`: Unix timestamp as date
- Custom display types are also supported

## Verifying Your NFTs

### Check Token Metadata
```bash
dfx canister call nft --network ic icrc7_token_metadata '(vec { 1 })'
```

### Check Token Owner
```bash
dfx canister call nft --network ic icrc7_owner_of '(vec { 1 })'
```

### Check Total Supply
```bash
dfx canister call nft --network ic icrc7_total_supply '()'
```

## CLI Tool Architecture

The CLI tool has been refactored into a modular architecture for better maintainability:

- **`main.rs`**: Entry point and command orchestration
- **`cli.rs`**: CLI argument definitions using clap
- **`commands.rs`**: Command execution handlers
- **`metadata.rs`**: ICRC97 metadata creation and validation
- **`prompts.rs`**: Interactive user input functions
- **`calls/`**: Canister interaction modules
- **`utils.rs`**: Utility functions and agent initialization

## Troubleshooting

### Common Issues:

1. **Permission Denied**: Ensure your identity is in the `authorized_principals` list
2. **Invalid Metadata**: Use the `validate-metadata` command to check your JSON
3. **Upload Failures**: Check network connectivity and canister cycles
4. **Minting Failures**: Verify you're in the `minting_authorities` list

### Getting Help:
```bash
../target/release/origyn_icrc7_cmdlinetools --help
../target/release/origyn_icrc7_cmdlinetools <subcommand> --help
```

## Additional Resources

- [Internet Computer Documentation](https://internetcomputer.org/docs/current/developer-docs/)
- [ICRC-7 Standard](https://github.com/dfinity/ICRC-7)
- [ICRC97 Metadata Standard](https://github.com/dfinity/ICRC/blob/main/ICRCs/ICRC-97/ICRC-97.md)