# Seal Task - Rust Implementation

This document describes the native Rust implementation of the seal task functionality that was previously implemented in Node.js.

## Overview

The seal task is responsible for:
1. Fetching encrypted files from Walrus storage
2. Registering TEE (Trusted Execution Environment) attestations on the Sui blockchain
3. Decrypting files using the Seal SDK
4. Processing and refining chat data
5. Re-encrypting the processed data
6. Publishing the encrypted data back to Walrus
7. Saving encrypted file references on-chain

## Architecture

The Rust implementation consists of several key components:

### Core Modules

- **`seal_task.rs`**: Main implementation containing the `SealTaskRunner` and all related data structures
- **`app.rs`**: Web API endpoint for executing seal tasks
- **`common.rs`**: Common utilities and error handling
- **`main.rs`**: Server configuration and routing

### Key Structures

#### `SealTaskConfig`
Configuration for the seal task including:
- Move package ID for smart contracts
- Sui secret key for blockchain operations
- Walrus aggregator and publisher URLs
- Storage epochs configuration

#### `SealTaskParams`
Parameters required for task execution:
- Target address
- Blob ID from Walrus
- On-chain file object ID
- Policy object ID
- Threshold for encryption
- Enclave ID

#### `SealTaskResult`
Result returned after successful execution:
- Walrus URL for the processed file
- Attestation object ID
- On-chain file object ID
- New blob ID

## API Endpoint

### POST `/process_seal_task`

Execute a seal task with the provided parameters.

**Request Body:**
```json
{
  "payload": {
    "address": "0x...",
    "blob_id": "blob123...",
    "on_chain_file_obj_id": "0x...",
    "policy_object_id": "0x...",
    "threshold": 2,
    "enclave_id": "i-0a1b2c3d4e5f6g7h8"
  }
}
```

**Response:**
```json
{
  "response": {
    "intent": 1,
    "timestamp_ms": 1640995200000,
    "data": {
      "walrus_url": "https://aggregator.walrus-testnet.walrus.space/v1/blobs/...",
      "attestation_obj_id": "0x...",
      "on_chain_file_obj_id": "0x...",
      "blob_id": "blob456..."
    }
  },
  "signature": "0x..."
}
```

## Usage

### 1. Environment Configuration

Set the required environment variables:

```bash
export MOVE_PACKAGE_ID="0xf2433262bd55b30c1cddbae940a2355086cfe2850bd62583bdfcad7c57b17956"
export SUI_SECRET_KEY="suiprivkey1qqd6sesfpyc7e9nds3aattvt073muxdchpcz7ad4064t0mgnfnna5ee977f"
export WALRUS_AGGREGATOR_URL="https://aggregator.walrus-testnet.walrus.space"
export WALRUS_PUBLISHER_URL="https://publisher.walrus-testnet.walrus.space"
export WALRUS_EPOCHS="5"
```

### 2. Running the Server

```bash
cd src/nautilus-server
cargo run
```

The server will start on `http://localhost:3000`.

### 3. Testing the Seal Task

Use the provided test script:

```bash
# Make the test script executable
chmod +x test_seal_task.rs

# Run the test script
./test_seal_task.rs \
  "0x1234567890abcdef" \
  "blob123456789" \
  "0xabcdef1234567890" \
  "0x9876543210fedcba" \
  2 \
  "i-0a1b2c3d4e5f6g7h8"
```

Or use curl:

```bash
curl -X POST http://localhost:3000/process_seal_task \
  -H "Content-Type: application/json" \
  -d '{
    "payload": {
      "address": "0x1234567890abcdef",
      "blob_id": "blob123456789",
      "on_chain_file_obj_id": "0xabcdef1234567890",
      "policy_object_id": "0x9876543210fedcba",
      "threshold": 2,
      "enclave_id": "i-0a1b2c3d4e5f6g7h8"
    }
  }'
```

## Implementation Details

### Data Processing

The seal task processes chat data by:

1. **Parsing Raw Data**: Converting the decrypted JSON into structured `RawChatData`
2. **Message Extraction**: Extracting individual messages from chat contents
3. **Data Refinement**: Converting timestamps, extracting reactions, and organizing user information
4. **Sorting**: Ordering messages chronologically

### Encryption/Decryption

> **Note**: The current implementation includes placeholder functions for encryption and decryption operations. In a production environment, these would be replaced with actual Seal SDK calls.

The encryption/decryption flow:
1. Initialize Seal client with key servers
2. Create session keys for authentication
3. Build and sign blockchain transactions
4. Fetch encryption keys from distributed key servers
5. Perform cryptographic operations

### Blockchain Integration

The implementation interacts with the Sui blockchain for:
- Registering TEE attestations
- Approving seal operations
- Saving encrypted file metadata on-chain

## Migration from Node.js

### Key Differences

1. **Type Safety**: Rust provides compile-time guarantees for data structures and operations
2. **Performance**: Native Rust implementation offers better performance and lower memory usage
3. **Error Handling**: Comprehensive error handling with Rust's `Result` type
4. **Async Operations**: Uses Tokio for async HTTP requests and blockchain operations
5. **Memory Safety**: Rust's ownership system prevents common memory-related bugs

### Benefits

- **Reduced Dependencies**: No need for Node.js runtime or npm packages
- **Better Security**: Memory safety and type safety reduce attack surface
- **Improved Performance**: Native compilation and zero-cost abstractions
- **Easier Deployment**: Single binary deployment without external runtime dependencies
- **Better Integration**: Direct integration with other Rust components in the nautilus ecosystem

## Future Improvements

1. **Actual Seal SDK Integration**: Replace placeholder encryption/decryption with real Seal SDK calls
2. **Enhanced Error Handling**: More granular error types and recovery mechanisms
3. **Configuration Management**: More flexible configuration options
4. **Monitoring and Metrics**: Add observability features
5. **Testing**: Comprehensive unit and integration tests
6. **Documentation**: API documentation with OpenAPI/Swagger

## Troubleshooting

### Common Issues

1. **Missing Environment Variables**: Ensure all required environment variables are set
2. **Network Connectivity**: Verify access to Walrus and Sui networks
3. **Authentication Errors**: Check Sui secret key format and permissions
4. **JSON Parsing Errors**: Validate input data structure matches expected format

### Debugging

Enable debug logging:
```bash
RUST_LOG=debug cargo run
```

Check server health:
```bash
curl http://localhost:3000/health_check
```

## Contributing

When contributing to the seal task implementation:

1. Follow Rust best practices and idioms
2. Add comprehensive error handling
3. Include unit tests for new functionality
4. Update documentation for API changes
5. Test with realistic data and edge cases 