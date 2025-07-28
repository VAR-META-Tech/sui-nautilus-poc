// Copyright (c), Nautilus Team
// SPDX-License-Identifier: Apache-2.0

/// Seal Manager Contract for TEE Access Permission System
/// 
/// This contract manages user permissions for TEE enclaves to access encrypted files
/// without requiring user private keys. Users grant specific enclaves access to their
/// files through on-chain permission objects that can be verified cryptographically.

module app::seal_manager;

use enclave::enclave::Enclave;
use std::vector;

// Error codes
const EInvalidPermission: u64 = 1;
const ENotOwner: u64 = 2;
const EEnclaveNotAllowed: u64 = 3;
const EAlreadyExists: u64 = 4;
const ENotFound: u64 = 5;

/// TEE Access Permission object
/// Links a file ID to specific enclaves that are allowed to decrypt it
public struct TeeAccessPermission has key, store {
    id: UID,
    file_id: vector<u8>,        // Seal encryption ID
    owner: address,             // File owner who granted permission
    allowed_enclaves: vector<ID>, // List of enclave IDs allowed to access this file
    created_at: u64,            // Timestamp when permission was created
}

/// Events for tracking permission activities
public struct PermissionGranted has copy, drop {
    permission_id: ID,
    file_id: vector<u8>,
    owner: address,
    enclave_id: ID,
    timestamp: u64,
}

public struct PermissionRevoked has copy, drop {
    permission_id: ID,
    file_id: vector<u8>,
    owner: address,
    enclave_id: ID,
    timestamp: u64,
}

public struct SealApproved has copy, drop {
    permission_id: ID,
    file_id: vector<u8>,
    enclave_id: ID,
    timestamp: u64,
}

/// Grant TEE access to encrypted file
/// Creates a new permission object allowing specified enclave to decrypt the file
public fun grant_tee_access(
    file_id: vector<u8>,
    enclave_id: ID,
    ctx: &mut TxContext
): ID {
    let permission = TeeAccessPermission {
        id: object::new(ctx),
        file_id,
        owner: ctx.sender(),
        allowed_enclaves: vector::singleton(enclave_id),
        created_at: ctx.epoch_timestamp_ms(),
    };
    
    let permission_id = permission.id.to_inner();
    
    // Emit permission granted event
    sui::event::emit(PermissionGranted {
        permission_id,
        file_id,
        owner: ctx.sender(),
        enclave_id,
        timestamp: ctx.epoch_timestamp_ms(),
    });
    
    transfer::public_share_object(permission);
    permission_id
}

/// Seal protocol approval function (required by Seal protocol)
/// This is called by the TEE to get approval for decrypting a file
public entry fun seal_approve<T>(
    file_id: vector<u8>,
    permission: &TeeAccessPermission,
    enclave: &Enclave<T>,
    ctx: &TxContext
) {
    // Verify this is an internal approval call
    approve_internal(file_id, permission, enclave, ctx);
    
    // Emit seal approved event
    sui::event::emit(SealApproved {
        permission_id: permission.id.to_inner(),
        file_id,
        enclave_id: object::id(enclave),
        timestamp: ctx.epoch_timestamp_ms(),
    });
}

/// Internal helper function for approval logic
fun approve_internal<T>(
    file_id: vector<u8>,
    permission: &TeeAccessPermission,
    enclave: &Enclave<T>,
    _ctx: &TxContext
) {
    // Verify the file ID matches the permission
    assert!(permission.file_id == file_id, EInvalidPermission);
    
    // Verify the enclave is in the allowed list
    let enclave_id = object::id(enclave);
    assert!(vector::contains(&permission.allowed_enclaves, &enclave_id), EEnclaveNotAllowed);
}

/// Add enclave to existing permission
/// Only the permission owner can add new enclaves
public fun add_enclave_to_permission(
    permission: &mut TeeAccessPermission,
    enclave_id: ID,
    ctx: &mut TxContext
) {
    // Verify caller is the permission owner
    assert!(permission.owner == ctx.sender(), ENotOwner);
    
    // Check if enclave is already in the list
    assert!(!vector::contains(&permission.allowed_enclaves, &enclave_id), EAlreadyExists);
    
    // Add the enclave to allowed list
    vector::push_back(&mut permission.allowed_enclaves, enclave_id);
    
    // Emit permission granted event
    sui::event::emit(PermissionGranted {
        permission_id: permission.id.to_inner(),
        file_id: permission.file_id,
        owner: permission.owner,
        enclave_id,
        timestamp: ctx.epoch_timestamp_ms(),
    });
}

/// Remove enclave from permission
/// Only the permission owner can remove enclaves
public fun remove_enclave_from_permission(
    permission: &mut TeeAccessPermission,
    enclave_id: ID,
    ctx: &mut TxContext
) {
    // Verify caller is the permission owner
    assert!(permission.owner == ctx.sender(), ENotOwner);
    
    // Find and remove the enclave from allowed list
    let (found, index) = vector::index_of(&permission.allowed_enclaves, &enclave_id);
    assert!(found, ENotFound);
    
    vector::remove(&mut permission.allowed_enclaves, index);
    
    // Emit permission revoked event
    sui::event::emit(PermissionRevoked {
        permission_id: permission.id.to_inner(),
        file_id: permission.file_id,
        owner: permission.owner,
        enclave_id,
        timestamp: ctx.epoch_timestamp_ms(),
    });
}

/// Get permission information
/// Returns permission details for inspection
public fun get_permission_info(permission: &TeeAccessPermission): (vector<u8>, address, vector<ID>, u64) {
    (
        permission.file_id,
        permission.owner,
        permission.allowed_enclaves,
        permission.created_at
    )
}

/// Check if an enclave has permission to access a file
public fun has_permission(permission: &TeeAccessPermission, enclave_id: ID): bool {
    vector::contains(&permission.allowed_enclaves, &enclave_id)
}

/// Get the file ID associated with this permission
public fun file_id(permission: &TeeAccessPermission): vector<u8> {
    permission.file_id
}

/// Get the owner of this permission
public fun owner(permission: &TeeAccessPermission): address {
    permission.owner
}

/// Get all allowed enclaves for this permission
public fun allowed_enclaves(permission: &TeeAccessPermission): vector<ID> {
    permission.allowed_enclaves
}

/// Get creation timestamp
public fun created_at(permission: &TeeAccessPermission): u64 {
    permission.created_at
}

/// Revoke all permissions for a file (delete the permission object)
/// Only the owner can revoke all permissions
public fun revoke_all_permissions(
    permission: TeeAccessPermission,
    ctx: &TxContext
) {
    // Verify caller is the permission owner
    assert!(permission.owner == ctx.sender(), ENotOwner);
    
    // Destroy the permission object
    let TeeAccessPermission { id, .. } = permission;
    id.delete();
}

#[test_only]
use sui::test_scenario;

#[test]
fun test_grant_tee_access() {
    let mut scenario = test_scenario::begin(@0x1);
    
    // Create test data
    let file_id = b"test_file_123";
    let enclave_id = object::id_from_address(@0x2);
    
    // Grant access
    let _permission_id = grant_tee_access(file_id, enclave_id, scenario.ctx());
    
    scenario.next_tx(@0x1);
    
    // Verify permission was created
    let permission = scenario.take_shared<TeeAccessPermission>();
    let (returned_file_id, owner, allowed_enclaves, _created_at) = get_permission_info(&permission);
    
    assert!(returned_file_id == file_id, 0);
    assert!(owner == @0x1, 1);
    assert!(vector::length(&allowed_enclaves) == 1, 2);
    assert!(*vector::borrow(&allowed_enclaves, 0) == enclave_id, 3);
    assert!(has_permission(&permission, enclave_id), 4);
    
    test_scenario::return_shared(permission);
    scenario.end();
}

#[test]
fun test_add_remove_enclave() {
    let mut scenario = test_scenario::begin(@0x1);
    
    // Setup
    let file_id = b"test_file_456";
    let enclave_id_1 = object::id_from_address(@0x2);
    let enclave_id_2 = object::id_from_address(@0x3);
    
    // Grant initial access
    grant_tee_access(file_id, enclave_id_1, scenario.ctx());
    
    scenario.next_tx(@0x1);
    
    // Get permission object
    let mut permission = scenario.take_shared<TeeAccessPermission>();
    
    // Add second enclave
    add_enclave_to_permission(&mut permission, enclave_id_2, scenario.ctx());
    
    // Verify both enclaves have permission
    assert!(has_permission(&permission, enclave_id_1), 0);
    assert!(has_permission(&permission, enclave_id_2), 1);
    
    // Remove first enclave
    remove_enclave_from_permission(&mut permission, enclave_id_1, scenario.ctx());
    
    // Verify only second enclave has permission
    assert!(!has_permission(&permission, enclave_id_1), 2);
    assert!(has_permission(&permission, enclave_id_2), 3);
    
    test_scenario::return_shared(permission);
    scenario.end();
}

#[test]
#[expected_failure(abort_code = ENotOwner)]
fun test_unauthorized_add_enclave() {
    let mut scenario = test_scenario::begin(@0x1);
    
    // Grant access as user 1
    let file_id = b"test_file_789";
    let enclave_id = object::id_from_address(@0x2);
    grant_tee_access(file_id, enclave_id, scenario.ctx());
    
    scenario.next_tx(@0x99); // Switch to different user
    
    let mut permission = scenario.take_shared<TeeAccessPermission>();
    let new_enclave_id = object::id_from_address(@0x4);
    
    // This should fail - user 99 is not the owner
    add_enclave_to_permission(&mut permission, new_enclave_id, scenario.ctx());
    
    test_scenario::return_shared(permission);
    scenario.end();
}

#[test]
fun test_permission_getters() {
    let mut scenario = test_scenario::begin(@0x1);
    
    // Create permission
    let file_id = b"test_file_getters";
    let enclave_id = object::id_from_address(@0x2);
    grant_tee_access(file_id, enclave_id, scenario.ctx());
    
    scenario.next_tx(@0x1);
    
    let permission = scenario.take_shared<TeeAccessPermission>();
    
    // Test getter functions
    assert!(file_id(&permission) == file_id, 0);
    assert!(owner(&permission) == @0x1, 1);
    assert!(vector::length(&allowed_enclaves(&permission)) == 1, 2);
    assert!(*vector::borrow(&allowed_enclaves(&permission), 0) == enclave_id, 3);
    assert!(has_permission(&permission, enclave_id), 4);
    assert!(!has_permission(&permission, object::id_from_address(@0x99)), 5);
    
    test_scenario::return_shared(permission);
    scenario.end();
}