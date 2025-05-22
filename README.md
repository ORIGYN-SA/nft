# ICRC7/ICRC37 Implementation for Internet Computer

This repository contains the first complete and production-ready implementation of the ICRC7/ICRC37 NFT standard for the Internet Computer. This implementation is currently under review by the DFINITY Foundation for community validation.

## Overview

This project provides a complete solution for NFT management on the Internet Computer, consisting of three main components:

1. **Core NFT Canister**: A full implementation of the ICRC7/ICRC37 standard for NFT management
2. **Storage Canister**: A high-performance storage solution for NFT assets
3. **Integration Tests**: Comprehensive test suite ensuring reliability and correctness

## Key Features

- Full ICRC7/ICRC37 standard compliance
- Production-ready implementation
- Complete integration test coverage
- High-performance storage solution
- Transaction history using ICRC3 standard
- Certified HTTP asset serving
- Stable memory storage with heap caching
- Fine-grained access control for assets

## Components

### Core NFT Canister (`src/core_nft`)
The main NFT ledger implementation that handles all NFT operations according to the ICRC7/ICRC37 standard. It uses the ICRC3 standard for transaction history and can work with any storage solution.

[Read more about Core NFT Canister](./src/core_nft/README.md)

### Storage Canister (`src/storage_canister`)
A specialized storage solution that serves assets via HTTP endpoints. Unlike DFINITY's asset canisters, it uses stable memory for storage with an intelligent caching system in heap memory.

[Read more about Storage Canister](./src/storage_canister/README.md)

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

## Why This Implementation?

This is currently the only complete implementation of the ICRC7/ICRC37 standard that:
- Is fully compliant with the standard
- Has complete integration test coverage
- Is production-ready
- Is under review by DFINITY Foundation
- Uses modern IC features like certified HTTP and stable memory
- Provides a flexible storage solution

## Contributing

We welcome contributions! Please read our contributing guidelines and submit pull requests.

## License

[License information]

## Support

For support, please open an issue in the GitHub repository or contact us through `gautier.wojda@bity.com`. 