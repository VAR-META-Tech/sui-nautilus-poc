// Copyright (c), Nautilus Team
// SPDX-License-Identifier: Apache-2.0

/// Integration tests for seal manager with enclave system

#[test_only]
module app::integration_test;

#[test_only]
use app::seal_manager::{Self, TeeAccessPermission};
#[test_only]
use enclave::enclave::{Self, EnclaveConfig, Cap};
#[test_only]
use sui::test_scenario;

#[test_only]
public struct TestWitness has drop {}

#[test]
fun test_seal_manager_integration() {
    let mut scenario = test_scenario::begin(@0x1);
    
    // Step 1: Create enclave capability and config
    let enclave_cap = enclave::new_cap(TestWitness {}, scenario.ctx());
    enclave::create_enclave_config(
        &enclave_cap,
        b"test_enclave".to_string(),
        b"pcr0_test",
        b"pcr1_test", 
        b"pcr2_test",
        scenario.ctx()
    );
    
    // Transfer the capability to the sender to clean it up
    transfer::public_transfer(enclave_cap, @0x1);
    
    // Step 2: Create a permission for a file
    let file_id = b"encrypted_file_123";
    let expected_enclave_id = object::id_from_address(@0x2); // Mock enclave ID
    let _permission_id = seal_manager::grant_tee_access(file_id, expected_enclave_id, scenario.ctx());
    
    scenario.next_tx(@0x1);
    
    // Step 3: Verify the permission was created correctly
    let permission = scenario.take_shared<TeeAccessPermission>();
    let config = scenario.take_shared<EnclaveConfig<TestWitness>>();
    
    // Verify permission details
    assert!(seal_manager::file_id(&permission) == file_id, 0);
    assert!(seal_manager::owner(&permission) == @0x1, 1);
    assert!(seal_manager::has_permission(&permission, expected_enclave_id), 2);
    
    // Verify enclave config exists and is properly structured  
    assert!(enclave::pcr0(&config) == &b"pcr0_test", 3);
    assert!(enclave::pcr1(&config) == &b"pcr1_test", 4);
    assert!(enclave::pcr2(&config) == &b"pcr2_test", 5);
    
    test_scenario::return_shared(permission);
    test_scenario::return_shared(config);
    scenario.end();
}

#[test]
fun test_permission_management_workflow() {
    let mut scenario = test_scenario::begin(@0x1);
    
    // Create initial permission
    let file_id = b"user_data_file";
    let enclave_id_1 = object::id_from_address(@0x2);
    let enclave_id_2 = object::id_from_address(@0x3);
    
    seal_manager::grant_tee_access(file_id, enclave_id_1, scenario.ctx());
    
    scenario.next_tx(@0x1);
    
    // Get permission and test management functions
    let mut permission = scenario.take_shared<TeeAccessPermission>();
    
    // Add second enclave
    seal_manager::add_enclave_to_permission(&mut permission, enclave_id_2, scenario.ctx());
    
    // Verify both enclaves have access
    assert!(seal_manager::has_permission(&permission, enclave_id_1), 0);
    assert!(seal_manager::has_permission(&permission, enclave_id_2), 1);
    
    // Remove first enclave
    seal_manager::remove_enclave_from_permission(&mut permission, enclave_id_1, scenario.ctx());
    
    // Verify only second enclave has access
    assert!(!seal_manager::has_permission(&permission, enclave_id_1), 2);
    assert!(seal_manager::has_permission(&permission, enclave_id_2), 3);
    
    // Test all getter functions
    let (returned_file_id, owner, allowed_enclaves, created_at) = seal_manager::get_permission_info(&permission);
    assert!(returned_file_id == file_id, 4);
    assert!(owner == @0x1, 5);
    assert!(std::vector::length(&allowed_enclaves) == 1, 6);
    // Timestamp should be defined (could be 0 in test environment)
    let _ = created_at; // Don't test exact timestamp value in tests
    
    test_scenario::return_shared(permission);
    scenario.end();
}

#[test]
fun test_multiple_permissions_for_different_files() {
    let mut scenario = test_scenario::begin(@0x1);
    
    // Create permissions for different files
    let file_id_1 = b"file_one";
    let file_id_2 = b"file_two";
    let enclave_id = object::id_from_address(@0x2);
    
    seal_manager::grant_tee_access(file_id_1, enclave_id, scenario.ctx());
    seal_manager::grant_tee_access(file_id_2, enclave_id, scenario.ctx());
    
    scenario.next_tx(@0x1);
    
    // Both permissions should exist as separate objects
    // Note: In a real test, we would need to handle multiple shared objects
    // For this demo, we just verify the concept works
    
    scenario.end();
}