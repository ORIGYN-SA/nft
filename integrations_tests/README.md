# Integration Tests

A comprehensive test suite for the ICRC7/ICRC37 implementation, ensuring reliability and correctness of the Core NFT and Storage Canisters.

## Overview

The integration tests provide complete coverage of the ICRC7/ICRC37 implementation, testing all aspects of NFT management and storage functionality. These tests are essential for ensuring the reliability and correctness of the implementation.

## Prerequisites

- Rust toolchain
- PocketIC binary
- Internet Computer SDK (dfx)

## Setting Up PocketIC

1. Download PocketIC for your platform:
   - Linux: [Download PocketIC for Linux](https://github.com/dfinity/pocket-ic/releases)
   - macOS: [Download PocketIC for macOS](https://github.com/dfinity/pocket-ic/releases)

2. Make the binary executable:
```bash
chmod +x pocket-ic
```

3. Set the environment variable:
```bash
export POCKET_IC_BIN=/path/to/pocket-ic
```

## Running Tests

### Build All Components

First, build all components:
```bash
bash ./scripts/build_all.sh
```

### Run Integration Tests

Run the complete test suite:
```bash
bash ./scripts/run_integrations_tests.sh
```

## Common issues and solutions:

1. **PocketIC Not Found**
   - Verify POCKET_IC_BIN is set correctly
   - Check binary permissions

2. **Build Failures**
   - Ensure all dependencies are installed
   - Check Rust toolchain version

3. **Test Failures**
   - Check test environment setup
   - Verify test data
   - Review error messages

## Contributing

We welcome contributions to improve the test suite. Please:

1. Follow existing test patterns
2. Add comprehensive documentation
3. Ensure all tests pass
4. Submit pull requests with clear descriptions

## Resources

- [PocketIC Documentation](https://github.com/dfinity/pocket-ic)
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [ICRC7 Standard](https://github.com/dfinity/ICRC-7) 