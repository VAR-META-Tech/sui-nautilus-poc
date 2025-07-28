# TEE Access Permission System - Implementation Plan

## Project Overview

### Current State
- ✅ Existing Nautilus framework with TEE processing
- ✅ Enclave registration system (`move/enclave/enclave.move`)
- ✅ Telegram data processing (`move/telegram/telegram.move`)
- ✅ Seal encryption/decryption in TEE environment
- ❌ **Issue**: TEE requires user's private key for Seal decryption

### Goal
Implement general address-based permission system where:
- Users can grant any Sui address access to their encrypted files (not limited to TEE enclaves)
- Any authorized address can decrypt files using their own wallet (no user private keys needed)
- Permissions are managed on-chain and revocable
- System works with any application, not just TEE environment

### Expected Outcome
**"User signs a message granting permission for any address to decrypt their encrypted files"** - enabling flexible, secure, auditable access control that works across any application with any wallet.

---

## Implementation Phases

### Phase 1: Smart Contract Development

#### 1.1 Create General Access Manager Contract
- [x] Create `move/app/sources/seal_manager.move`
- [ ] **UPDATED**: Implement `FileAccessPermission` struct with fields:
  - [x] `file_id: vector<u8>` - Seal encryption ID
  - [x] `owner: address` - File owner
  - [ ] **CHANGE**: `allowed_addresses: vector<address>` - Any Sui addresses (not just enclaves)
  - [x] `created_at: u64` - Timestamp
- [ ] **UPDATED**: Implement `grant_file_access()` function (general address-based)
- [ ] **UPDATED**: Implement `seal_approve()` entry function (works with any address)
- [ ] **UPDATED**: Remove enclave dependency from approval logic
- [ ] **UPDATED**: Add general address permission management functions:
  - [ ] `add_address_to_permission()`
  - [ ] `remove_address_from_permission()`
  - [ ] `get_permission_info()`
  - [ ] `has_permission()` - Check if address has permission
  - [ ] `revoke_all_permissions()` - Permission management

#### 1.2 Contract Testing
- [ ] **UPDATED**: Write unit tests for address-based permission granting
- [ ] **UPDATED**: Write unit tests for general address access control logic
- [ ] **OPTIONAL**: Test integration with existing enclave system (backward compatibility)
- [ ] **UPDATED**: Test error handling and edge cases for address-based permissions
- [ ] **UPDATED**: Update comprehensive integration tests (`move/app/sources/integration_test.move`)
- [ ] **PENDING**: Test Results: TBD after contract update

#### 1.3 Contract Deployment
- [x] Update `move/app/Move.toml` with dependencies (no additional dependencies needed)
- [x] Verify contract functions work correctly (all builds and tests pass)
- [x] Deploy to Sui testnet
- [x] Document contract addresses and object IDs

**Phase 1 Status: 🔄 IN PROGRESS - UPDATING TO GENERAL ADDRESS MODEL**
- Original enclave-based contract: ✅ Complete
- **NEW**: General address-based contract: 🔄 In Progress
- Testing: ❌ Needs update for new address model
- Integration: ❌ Needs update for general use cases
- Ready for deployment: ❌ Pending contract updates

### Phase 2: General Application Integration Updates

#### 2.1 Update Seal Operations (Any Application)
- [ ] **UPDATED**: Create general `SealOperations` for any application
- [ ] **UPDATED**: Implement address-based decryption:
  - [ ] `decrypt_with_wallet_credentials()` method
  - [ ] Use caller's own address for `SessionKey`
  - [ ] Sign with caller's private key (any wallet)
  - [ ] Create `seal_approve` transaction with permission object
- [ ] **SAME**: Remove user private key dependencies from file owner
- [ ] **UPDATED**: Add permission object parameter to general decryption flow

#### 2.2 Update Sui Operations (General Use)
- [ ] **UPDATED**: Add `create_seal_approve_transaction()` method
- [ ] **UPDATED**: Update transaction building for any address as sender
- [ ] **SAME**: Add permission verification helpers
- [ ] **UPDATED**: Test transaction creation and execution with any wallet

#### 2.3 Integration with Applications
- [ ] **UPDATED**: Update any application to use new decryption flow
- [ ] **UPDATED**: Modify operations to pass permission objects
- [ ] **UPDATED**: Update message retrieval to use authorized address credentials
- [ ] **UPDATED**: Test all operation types with new general system

### Phase 3: General Client Integration

#### 3.1 Permission Granting UI (Any Application)
- [ ] **UPDATED**: Add permission granting interface to any client app
- [ ] **UPDATED**: Create permission granting form with:
  - [ ] **CHANGE**: Sui address input (any wallet address, not just enclaves)
  - [ ] **SAME**: File ID input
  - [ ] **SAME**: Permission confirmation
- [ ] **UPDATED**: Implement smart contract call for `grant_file_access()`
- [ ] **SAME**: Add success/error feedback for permission granting

#### 3.2 Upload Flow Enhancement (General Use)
- [ ] **UPDATED**: Modify file upload flow to include address permission step
- [ ] **UPDATED**: Show authorized address information to users
- [ ] **SAME**: Add permission status indicators
- [ ] **SAME**: Implement permission management (view/revoke)

#### 3.3 API Integration (Any Application)
- [ ] **OPTIONAL**: Verify existing `/get_attestation` endpoint works for backward compatibility
- [ ] **UPDATED**: Update processing endpoints to accept permission objects (any application)
- [ ] **SAME**: Add error handling for permission-related failures
- [ ] **UPDATED**: Test client-to-any-address permission flow

### Phase 4: Testing & Deployment

#### 4.1 End-to-End Testing (General Use)
- [ ] **UPDATED**: Test complete user flow:
  - [ ] **CHANGE**: File upload → Permission granting → Any address processing → Results
- [ ] **SAME**: Test permission management:
  - [ ] **SAME**: Grant → Revoke → Re-grant permissions
- [ ] **UPDATED**: Test error scenarios:
  - [ ] **SAME**: Invalid permissions
  - [ ] **SAME**: Expired or revoked access
  - [ ] **CHANGE**: Wrong address attempting access
- [ ] **UPDATED**: Performance testing with multiple files and addresses

#### 4.2 Security Testing (General Address Model)
- [ ] **UPDATED**: Verify any address cannot access files without permission
- [ ] **SAME**: Test permission isolation between users
- [ ] **UPDATED**: Validate address-based identity verification
- [ ] **SAME**: Audit smart contract for vulnerabilities
- [ ] **SAME**: Test against replay attacks and manipulation

#### 4.3 Production Deployment (General System)
- [ ] **SAME**: Deploy smart contracts to Sui mainnet
- [ ] **UPDATED**: Update any applications with new code (not just TEE)
- [ ] **SAME**: Update client applications
- [ ] **UPDATED**: Create deployment documentation for general use
- [ ] **SAME**: Set up monitoring and alerting

---

## Technical Specifications

### Smart Contract Interface (Updated - General Address Model)

```move
// Grant any address access to encrypted file
public fun grant_file_access(
    file_id: vector<u8>,    // Seal encryption ID  
    allowed_address: address, // Any Sui address (not just enclaves)
    ctx: &mut TxContext
): ID

// Seal protocol approval function (simplified - no enclave dependency)
public entry fun seal_approve(
    file_id: vector<u8>,                // Requested encryption ID
    permission: &FileAccessPermission,  // Access permission object
    ctx: &TxContext                     // Caller must be in allowed_addresses
)
```

### General Application Interface (Updated)

```rust
// Decrypt using any wallet credentials and permission
async fn decrypt_with_wallet_credentials(
    file_object_id: &str,
    permission_object_id: &str, 
    encrypted_file: &[u8],
    wallet_address: &str  // Any Sui address
) -> Result<serde_json::Value>

// Create seal approve transaction for any address
async fn create_seal_approve_transaction(
    file_object_id: &str,
    permission_object_id: &str,
    caller_address: &str  // Any authorized address
) -> Result<Vec<u8>>
```

### Client API Interface (Updated - General Address)

```javascript
// Grant any address access (client-side)
async function grantFileAccess(fileId, targetAddress) {
    // Smart contract call to grant_file_access
    return permissionObjectId;
}

// Get user's own address (any wallet)
async function getWalletAddress() {
    // Returns current wallet address for permission granting
    return walletAddress;
}
```

---

## User Flow Diagram (Updated - General Address Model)

```
1. User uploads file
   ↓
2. File encrypted and stored in Walrus  
   ↓
3. User gets target address (any Sui wallet address)
   ↓
4. User calls grant_file_access(file_id, target_address)
   ↓
5. Permission object created on-chain
   ↓
6. Any authorized address processes file using permission object
   ↓
7. Authorized address decrypts with its own wallet credentials
   ↓
8. Authorized address submits results (any application flow)
```

---

## Testing Scenarios

### Positive Test Cases (Updated - General Address)
- [ ] **UPDATED**: User grants permission → Any authorized address successfully decrypts
- [ ] **UPDATED**: Multiple addresses can access same file with separate permissions
- [ ] **SAME**: User can revoke and re-grant permissions
- [ ] **SAME**: Permission works across different file types

### Negative Test Cases (Updated - General Address)
- [ ] **UPDATED**: Any address cannot decrypt without permission
- [ ] **UPDATED**: Wrong address cannot access file even with different permission
- [ ] **SAME**: Revoked permissions prevent access
- [ ] **SAME**: Invalid permission objects are rejected

### Edge Cases (Updated - General Address)
- [ ] **SAME**: Permission object deletion/corruption
- [ ] **UPDATED**: Address reuse with different wallets
- [ ] **SAME**: High-volume permission granting
- [ ] **UPDATED**: Concurrent access from multiple addresses

---

## Success Criteria

### Functional Requirements (Updated - General Address)
- [ ] **UPDATED**: Users can grant any address access via simple UI interaction
- [ ] **UPDATED**: Any authorized address can decrypt files without user private keys
- [ ] **SAME**: All existing functionality (embedding, retrieval, processing) works
- [ ] **SAME**: Permissions are revocable and auditable on-chain

### Security Requirements (Updated - General Address)
- [ ] **UPDATED**: No user private keys stored in any application environment
- [ ] **SAME**: Access control is enforced cryptographically
- [ ] **SAME**: Unauthorized access attempts are blocked
- [ ] **SAME**: All access is logged and auditable

### Performance Requirements (Updated - General Address)
- [ ] **SAME**: Permission granting completes within 10 seconds
- [ ] **UPDATED**: Address-based decryption performance unchanged
- [ ] **SAME**: UI remains responsive during permission operations
- [ ] **SAME**: System handles 100+ concurrent permission grants

---

## Risk Mitigation

### Technical Risks
- **Risk**: Seal protocol changes break compatibility
- **Mitigation**: Use official examples, test extensively, maintain version compatibility

- **Risk**: Smart contract vulnerabilities
- **Mitigation**: Thorough testing, security audit, gradual rollout

### Operational Risks  
- **Risk**: User confusion with permission model
- **Mitigation**: Clear UI/UX, comprehensive documentation, user testing

- **Risk**: TEE deployment complexity  
- **Mitigation**: Automated deployment scripts, comprehensive testing, rollback procedures

---

## Timeline Estimate

- **Phase 1 (Smart Contracts)**: 1-2 weeks
- **Phase 2 (TEE Services)**: 2-3 weeks  
- **Phase 3 (Client Integration)**: 1-2 weeks
- **Phase 4 (Testing & Deployment)**: 1-2 weeks

**Total Estimated Time**: 5-9 weeks

---

## Dependencies

### External Dependencies
- Sui blockchain network availability
- Mysten Seal protocol stability  
- AWS Nitro Enclave infrastructure

### Internal Dependencies
- Existing enclave registration system
- Current Telegram data processing contracts
- Client application framework

---

## Rollback Plan

### Smart Contract Rollback
- [ ] Maintain backward compatibility during transition
- [ ] Keep old permission system functional
- [ ] Gradual migration with feature flags

### TEE Service Rollback
- [ ] Maintain dual decryption paths (old + new)
- [ ] Environment variables to switch between modes
- [ ] Quick revert to user private key mode if needed

### Client Rollback  
- [ ] Feature flags for permission UI
- [ ] Fallback to direct processing mode
- [ ] User notification of system changes

---

## Monitoring & Alerting

### Smart Contract Monitoring
- [ ] Permission granting success/failure rates
- [ ] Access approval/denial metrics  
- [ ] Gas usage optimization

### TEE Service Monitoring
- [ ] Decryption success rates with new system
- [ ] Performance metrics comparison
- [ ] Error rate tracking

### User Experience Monitoring
- [ ] Permission granting completion rates
- [ ] User abandonment at permission step
- [ ] Support ticket volume for permission issues

---

## Implementation Progress Log

### Phase 1 Completion - January 28, 2025

**Smart Contract Development: ✅ COMPLETED**

#### Files Created:
- `move/app/sources/seal_manager.move` - Main permission management contract
- `move/app/sources/integration_test.move` - Integration tests with enclave system

#### Key Features Implemented:
1. **TeeAccessPermission Struct** - Complete permission object with all required fields
2. **Permission Management Functions**:
   - `grant_tee_access()` - Create permissions for specific enclaves
   - `seal_approve()` - Seal protocol compliance function
   - `add_enclave_to_permission()` / `remove_enclave_from_permission()` - Dynamic permission management
   - `has_permission()`, `get_permission_info()` - Permission queries
   - `revoke_all_permissions()` - Complete permission revocation

3. **Security & Access Control**:
   - Owner-only permission modifications
   - Enclave ID verification for access
   - Event emission for all permission activities

4. **Testing & Validation**:
   - 7 comprehensive unit and integration tests (all passing)
   - Full integration with existing enclave system
   - Error handling and edge case coverage

#### Technical Achievements:
- ✅ Contract builds successfully with `sui move build`
- ✅ All tests pass with `sui move test`
- ✅ Integration with existing `enclave::enclave` module confirmed
- ✅ Event-driven architecture for monitoring
- ✅ No additional dependencies required

#### Next Steps:
Phase 1 is **production-ready** for Sui testnet deployment. Phase 2 (TEE Service Updates) can now begin development using the completed smart contract foundation.

---

*This plan will be updated as implementation progresses and requirements are refined.*