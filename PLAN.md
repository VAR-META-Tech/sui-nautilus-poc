# TEE Access Permission System - Implementation Plan

## Project Overview

### Current State
- ✅ Existing Nautilus framework with TEE processing
- ✅ Enclave registration system (`move/enclave/enclave.move`)
- ✅ Telegram data processing (`move/telegram/telegram.move`)
- ✅ Seal encryption/decryption in TEE environment
- ❌ **Issue**: TEE requires user's private key for Seal decryption

### Goal
Implement user-controlled permission system where:
- Users can grant specific TEE enclaves access to their encrypted files
- TEE can decrypt files using its own keypair (no user private keys needed)
- Permissions are managed on-chain and revocable

### Expected Outcome
**"User signs a message granting permission for TEE to use their message"** - enabling secure, auditable access control without compromising user's private keys.

---

## Implementation Phases

### Phase 1: Smart Contract Development

#### 1.1 Create Seal Manager Contract
- [x] Create `move/app/sources/seal_manager.move`
- [x] Implement `TeeAccessPermission` struct with fields:
  - [x] `file_id: vector<u8>` - Seal encryption ID
  - [x] `owner: address` - File owner
  - [x] `allowed_enclaves: vector<ID>` - Allowed enclave IDs
  - [x] `created_at: u64` - Timestamp
- [x] Implement `grant_tee_access()` function
- [x] Implement `seal_approve()` entry function (required by Seal protocol)
- [x] Implement `approve_internal()` helper function
- [x] Add permission management functions:
  - [x] `add_enclave_to_permission()`
  - [x] `remove_enclave_from_permission()`
  - [x] `get_permission_info()`
  - [x] `has_permission()` - Additional helper function
  - [x] `revoke_all_permissions()` - Additional management function

#### 1.2 Contract Testing
- [x] Write unit tests for permission granting
- [x] Write unit tests for access control logic
- [x] Test integration with existing enclave system
- [x] Test error handling and edge cases
- [x] Create comprehensive integration tests (`move/app/sources/integration_test.move`)
- [x] **Test Results**: 7 tests passed, 0 failed

#### 1.3 Contract Deployment
- [x] Update `move/app/Move.toml` with dependencies (no additional dependencies needed)
- [x] Verify contract functions work correctly (all builds and tests pass)
- [ ] Deploy to Sui testnet
- [ ] Document contract addresses and object IDs

**Phase 1 Status: ✅ COMPLETED**
- Contract implementation: ✅ Complete
- Testing: ✅ Complete (7/7 tests passing)
- Integration: ✅ Complete (with existing enclave system)
- Ready for deployment: ✅ Yes

### Phase 2: TEE Service Updates

#### 2.1 Update Seal Operations (Native Rust)
- [ ] Create new `SealOperations` struct in Rust services
- [ ] Implement TEE credential-based decryption:
  - [ ] `decrypt_with_tee_credentials()` method
  - [ ] Use TEE's own address for `SessionKey`
  - [ ] Sign with TEE's private key
  - [ ] Create `seal_approve` transaction with permission object
- [ ] Remove user private key dependencies
- [ ] Add permission object parameter to decryption flow

#### 2.2 Update Sui Operations (Native Rust)
- [ ] Add `create_seal_approve_transaction()` method
- [ ] Update transaction building for TEE as sender
- [ ] Add permission verification helpers
- [ ] Test transaction creation and execution

#### 2.3 Integration with Existing Services
- [ ] Update task operations to use new decryption flow
- [ ] Modify embedding operations to pass permission objects
- [ ] Update message retrieval to use TEE credentials
- [ ] Test all operation types with new system

### Phase 3: Client Integration

#### 3.1 Permission Granting UI
- [ ] Add permission granting interface to client app
- [ ] Create permission granting form with:
  - [ ] Enclave ID selection (from `/get_attestation`)
  - [ ] File ID input
  - [ ] Permission confirmation
- [ ] Implement smart contract call for `grant_tee_access()`
- [ ] Add success/error feedback for permission granting

#### 3.2 Upload Flow Enhancement
- [ ] Modify file upload flow to include permission step
- [ ] Show enclave information to users
- [ ] Add permission status indicators
- [ ] Implement permission management (view/revoke)

#### 3.3 API Integration
- [ ] Verify existing `/get_attestation` endpoint works for enclave info
- [ ] Update processing endpoints to accept permission objects
- [ ] Add error handling for permission-related failures
- [ ] Test client-to-TEE permission flow

### Phase 4: Testing & Deployment

#### 4.1 End-to-End Testing
- [ ] Test complete user flow:
  - [ ] File upload → Permission granting → TEE processing → Results
- [ ] Test permission management:
  - [ ] Grant → Revoke → Re-grant permissions
- [ ] Test error scenarios:
  - [ ] Invalid permissions
  - [ ] Expired or revoked access
  - [ ] Wrong enclave attempting access
- [ ] Performance testing with multiple files and enclaves

#### 4.2 Security Testing
- [ ] Verify TEE cannot access files without permission
- [ ] Test permission isolation between users
- [ ] Validate enclave identity verification
- [ ] Audit smart contract for vulnerabilities
- [ ] Test against replay attacks and manipulation

#### 4.3 Production Deployment
- [ ] Deploy smart contracts to Sui mainnet
- [ ] Update all TEE instances with new code
- [ ] Update client applications
- [ ] Create deployment documentation
- [ ] Set up monitoring and alerting

---

## Technical Specifications

### Smart Contract Interface

```move
// Grant TEE access to encrypted file
public fun grant_tee_access(
    file_id: vector<u8>,    // Seal encryption ID  
    enclave_id: ID,         // Target enclave ID
    ctx: &mut TxContext
): ID

// Seal protocol approval function
public entry fun seal_approve(
    id: vector<u8>,                    // Requested encryption ID
    permission: &TeeAccessPermission,   // Access permission object
    enclave: &Enclave<TELEGRAM>,       // Requesting enclave
    ctx: &TxContext
)
```

### TEE Service Interface

```rust
// Decrypt using TEE credentials and permission
async fn decrypt_with_tee_credentials(
    file_object_id: &str,
    permission_object_id: &str, 
    encrypted_file: &[u8]
) -> Result<serde_json::Value>

// Create seal approve transaction for TEE
async fn create_seal_approve_transaction(
    file_object_id: &str,
    permission_object_id: &str,
    enclave_id: &str
) -> Result<Vec<u8>>
```

### Client API Interface

```javascript
// Grant TEE access (client-side)
async function grantTeeAccess(fileId, enclaveId) {
    // Smart contract call to grant_tee_access
    return permissionObjectId;
}

// Get enclave information
GET /get_attestation
// Returns: { attestation: { enclaveId: "..." } }
```

---

## User Flow Diagram

```
1. User uploads file
   ↓
2. File encrypted and stored in Walrus  
   ↓
3. User gets enclave ID from /get_attestation
   ↓
4. User calls grant_tee_access(file_id, enclave_id)
   ↓
5. Permission object created on-chain
   ↓
6. TEE processes file using permission object
   ↓
7. TEE decrypts with its own credentials
   ↓
8. TEE submits results via mark_processed()
```

---

## Testing Scenarios

### Positive Test Cases
- [ ] User grants permission → TEE successfully decrypts
- [ ] Multiple enclaves can access same file with separate permissions
- [ ] User can revoke and re-grant permissions
- [ ] Permission works across different file types

### Negative Test Cases  
- [ ] TEE cannot decrypt without permission
- [ ] Wrong enclave cannot access file even with different permission
- [ ] Revoked permissions prevent access
- [ ] Invalid permission objects are rejected

### Edge Cases
- [ ] Permission object deletion/corruption
- [ ] Enclave re-registration with same ID
- [ ] High-volume permission granting
- [ ] Concurrent access from multiple TEEs

---

## Success Criteria

### Functional Requirements
- [ ] Users can grant TEE access via simple UI interaction
- [ ] TEE can decrypt files without user private keys
- [ ] All existing functionality (embedding, retrieval, processing) works
- [ ] Permissions are revocable and auditable on-chain

### Security Requirements  
- [ ] No user private keys stored in TEE environment
- [ ] Access control is enforced cryptographically
- [ ] Unauthorized access attempts are blocked
- [ ] All access is logged and auditable

### Performance Requirements
- [ ] Permission granting completes within 10 seconds
- [ ] TEE decryption performance unchanged
- [ ] UI remains responsive during permission operations
- [ ] System handles 100+ concurrent permission grants

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