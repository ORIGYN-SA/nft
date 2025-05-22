# Storage Canister

A high-performance storage solution for the Internet Computer that serves assets via HTTP endpoints with certified responses.

## Overview

The Storage Canister is a specialized component designed to efficiently store and serve assets on the Internet Computer. Unlike DFINITY's asset canisters, it uses stable memory for storage with an intelligent caching system in heap memory, providing better performance and reliability.

## Key Features

- **Stable Memory Storage**: Assets are primarily stored in stable memory, ensuring persistence across canister upgrades
- **Intelligent Caching**: Implements a sophisticated caching system in heap memory for frequently accessed assets
- **Certified HTTP Responses**: Serves assets with certified HTTP responses for secure content delivery
- **Cache Miss Handling**: Automatically handles cache misses by fetching from stable memory
- **Memory Management**: Efficient memory management with automatic cleanup of heap memory
- **High Performance**: Optimized for serving assets with minimal latency

## How It Works

1. **Storage**: Assets are stored in stable memory for persistence
2. **Caching**: Frequently accessed assets are cached in heap memory for quick access
3. **Request Flow**:
   - When a request comes in, the canister first checks the heap cache
   - If the asset is in cache (cache hit), it's served immediately
   - If not in cache (cache miss), the canister:
     1. Converts the query call to an update call
     2. Frees space in heap memory if needed
     3. Retrieves the asset from stable memory
     4. Serves the asset with a certified HTTP response
4. **Error Handling**: If the asset is not available, an appropriate error is returned

## Contributing

We welcome contributions to improve the Storage Canister. Please read our contributing guidelines before submitting pull requests. 