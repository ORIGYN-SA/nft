# Core NFT Canister

A complete implementation of the ICRC7/ICRC37 NFT standard for the Internet Computer, providing a robust and flexible NFT ledger solution.

## Overview

The Core NFT Canister is the first complete implementation of the ICRC7/ICRC37 standard for the Internet Computer. It provides a full-featured NFT ledger with transaction history using the ICRC3 standard and flexible storage options.

## Key Features

- **Full ICRC7/ICRC37 Compliance**: Complete implementation of the NFT standard
- **Transaction History**: Integrated with ICRC3 for comprehensive transaction tracking
- **Flexible Storage**: Can work with any storage solution, including our Storage Canister
- **Certified Responses**: All queries are certified for security
- **Comprehensive Testing**: Complete integration test coverage
- **Production Ready**: Currently under review by DFINITY Foundation

## Architecture

### Components

1. **NFT Ledger**: Core functionality for NFT management
2. **Transaction History**: ICRC3 integration for transaction tracking
3. **Storage Interface**: Flexible storage abstraction
4. **Access Control**: Fine-grained permissions system

### Storage Integration

The Core NFT Canister is designed to work with any storage solution. By default, it integrates with our Storage Canister for asset management, but you can implement your own storage solution by implementing the storage interface.

## Usage

See ./example directory, where a complete example is provided

## ICRC3 Integration

The Core NFT Canister uses the ICRC3 standard for transaction history. This provides:

- Standardized transaction format
- Efficient storage and retrieval
- Certified queries
- Automatic archiving

## Testing

The Core NFT Canister includes comprehensive integration tests. Run the tests using:

```bash
bash ./scripts/run_integrations_tests.sh
```

## Contributing

We welcome contributions to improve the Core NFT Canister. Please read our contributing guidelines before submitting pull requests.

## Resources

- [ICRC7 Standard](https://github.com/dfinity/ICRC-7)
- [ICRC3 Implementation](https://github.com/BitySA/dfinity-rust-libraries/tree/master/src/icrc3)
- [DFINITY Forum Post](https://forum.dfinity.org/t/introducing-bity-ic-icrc3-a-complete-icrc3-implementation-for-internet-computer-developers/44520) 