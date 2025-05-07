# ICRC7 NFT Implementation

This project implements an NFT (Non-Fungible Token) standard on the Internet Computer (IC) blockchain, following the ICRC7 specification. It provides a robust and scalable solution for managing digital assets on the IC network.

## Project Structure

The project is organized as a Rust workspace with multiple components:

- `src/core_nft`: Core NFT implementation
- `src/storage_canister`: Storage management for NFT assets
- `src/storage_api_canister`: API interface for storage operations
- `src/external_canisters`: External canister integrations
- `integrations_tests`: Integration test suite

## Features

- ICRC7 compliant NFT implementation
- Secure storage management
- Integration with Internet Computer's native features
- Comprehensive test coverage

## Prerequisites

- Rust (latest stable version)
- DFX (Internet Computer SDK)
- Git
- Pocket-IC (for integration tests)
- Candid-extractor

## Getting Started

1. Clone the repository:
```bash
git clone [repository-url]
cd icrc7_nft
```

2. Build the project:
```bash
bash ./scripts/build_all.sh
```

This script will:
- Build the storage canister
- Build the core NFT canister

You can get your canister wasm, wasm.gz, and candid definition here :
`./src/core_nft/wasm/`
`./src/storage_canister/wasm/`

## Development

The project uses Rust's workspace feature to manage multiple crates. Each component can be developed and tested independently.

### Running Tests

1. Set the Pocket-IC binary path:
```bash
export POCKET_IC_BIN=/usr/local/bin/pocket-ic
```

2. Run the integration tests:
```bash
bash ./scripts/run_integrations_tests.sh
```

This will:
- Build all canisters
- Copy the core NFT canister WASM to the integration tests
- Run the integration test suite

## Deployment

Deployment instructions for both local and mainnet environments will be added soon.

## Management CLI

COMING SOON

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request, or contact contact@origyn.ch

## License

[Add your license information here]

## Contact

[Add your contact information here]
