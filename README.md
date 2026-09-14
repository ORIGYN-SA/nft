# ICRC7/ICRC37 Implementation for Internet Computer

This repository contains the first complete and production-ready implementation of the ICRC7/ICRC37 NFT standard for the Internet Computer. This implementation is currently under review by the DFINITY Foundation for community validation.

## Overview

This project provides a complete solution for NFT management on the Internet Computer, consisting of three main components:

**Core NFT Canister**: A full implementation of the ICRC7/ICRC37 standard for NFT management
**Integration Tests**: Comprehensive test suite ensuring reliability and correctness

## Key Features

- Full ICRC7/ICRC37 standard compliance
- Production-ready implementation
- Complete integration test coverage
- High-performance storage solution
- Transaction history using ICRC3 standard
- Certified HTTP asset serving
- Stable memory storage with heap caching
- Fine-grained access control for assets
- Storage of private content using VetKeys

## Components

### Core NFT Canister (`src/core_nft`)
The main NFT ledger implementation that handles all NFT operations according to the ICRC7/ICRC37 standard. It uses the ICRC3 standard for transaction history and can work with any storage solution.

[Read more about Core NFT Canister](./src/core_nft/README.md)

### Integration Tests (`integrations_tests`)
A comprehensive test suite that ensures the reliability and correctness of the implementation. The tests cover all aspects of the NFT standard and storage functionality.

[Read more about Integration Tests](./integrations_tests/README.md)

## Getting Started

### Prerequisites

- Internet Computer SDK (dfx)
- PocketIC for running integration tests
- Rust toolchain

### Installation

1. Clone the repository:
```bash
git clone https://github.com/ORIGYN-SA/nft.git
cd icrc7
```

2. Build the project:
```bash
bash ./scripts/build_all.sh
```

3. Run integration tests:
```bash
export POCKET_IC_BIN=/path/to/pocket-ic
bash ./scripts/run_integrations_tests.sh
```

## Upgrading

A collection canister and its storage sub-canisters are upgraded together, and
the order matters.

**Storage sub-canisters must be on 0.7.0 before any 0.7 shaped `init_upload`
reaches them.** In 0.7 the `file_hash` argument became `opt text`. A
sub-canister still running 0.6.1 declares it as `text` and rejects the call, and
the collection cannot tell that rejection apart from "this canister is full":
`StorageSubCanisterManager::init_upload` treats any error from a sub-canister as
a reason to try the next one, and creates a brand new storage canister when none
accepts. A fleet left on 0.6.1 therefore does not fail loudly; it grows one new
storage canister per upload.

The collection upgrades its own fleet. `post_upgrade` schedules that on a zero
delay timer, because inter-canister calls are forbidden inside `post_upgrade`
itself. The timer logs failures and does not retry them, so the upgrade is not
finished until you have checked the result.

After upgrading the collections (`upgrade_collections` on the managing canister,
or a direct `dfx canister install --mode upgrade`):

1. Read the collection logs and confirm no `Storage canister upgrade failed`
   entry appears. Each such line is one sub-canister still on the old wasm.
2. Confirm that every storage sub-canister's module hash equals the sha256 of
   `wasm/storage_canister.wasm.gz`, the wasm this build embeds:

   ```bash
   shasum -a 256 wasm/storage_canister.wasm.gz
   dfx canister --network ic info <storage_canister_id>   # Module hash: 0x...
   ```

   The collection installs the gzipped bytes, so the module hash is the sha256
   of the `.gz` file, not of the uncompressed wasm.

Recovery is to run the upgrade again: every `post_upgrade` reschedules the
timer, so a second upgrade retries every sub-canister that is still behind. Do
not let 0.7 uploads reach the collection until both checks pass.

## Why This Implementation?

This is currently the only complete implementation of the ICRC7/ICRC37 standard that:
- Is fully compliant with the standard
- Has complete integration test coverage
- Is production-ready
- Is under review by DFINITY Foundation
- Uses modern IC features like certified HTTP and stable memory
- Provides a flexible storage solution

## This project also use 

### Storage Canister

[Read more about Storage Canister](https://gitlab.bity.com/bity/dev/icp/storage-canister)

### ICRC3 library

[Read more about ICRC3 library](https://github.com/BitySA/dfinity-rust-libraries/tree/master/src/icrc3)


## Contributing

We welcome contributions! Please read our contributing guidelines and submit pull requests.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](./LICENSE) for the full text,
or obtain a copy at <http://www.apache.org/licenses/LICENSE-2.0>.

## Support

For support, please open an issue in the GitHub repository or contact us through `gautier.wojda@bity.com`. 