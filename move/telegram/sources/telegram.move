// Copyright (c), Nautilus Team
// SPDX-License-Identifier: Apache-2.0

module telegram::telegram;

use enclave::enclave::{Self, Enclave};
use std::string::String;

/// ====
/// Telegram DataDAO smart contract for processing encrypted Telegram messages
/// using TEE (Trusted Execution Environment) with Walrus storage and Seal encryption
/// ====

// Intent scope for Telegram processing
const TELEGRAM_PROCESSING_INTENT: u8 = 1;

// Error codes
const EInvalidSignature: u64 = 1;
const EAlreadyProcessed: u64 = 2;
const ENotAuthorized: u64 = 3;

// Telegram submission record - tracks data uploaded by users
public struct TelegramSubmission has key {
    id: UID,
    blob_id: vector<u8>,        // Walrus blob ID of encrypted data
    submitter: address,          // User who submitted the data
    timestamp: u64,              // Submission timestamp
    processed: bool,             // Whether TEE has processed this data
    processing_result_blob_id: Option<vector<u8>>, // Result blob ID after processing
}

// Processing response from TEE - used for signature verification
public struct TelegramProcessingResponse has copy, drop {
    original_blob_id: vector<u8>,
    processed_blob_id: vector<u8>,
    message_count: u64,
    processing_metadata: String,
}

// Witness type for creating enclave capability
public struct TELEGRAM has drop {}

// Events for tracking
public struct DataSubmitted has copy, drop {
    submission_id: ID,
    blob_id: vector<u8>,
    submitter: address,
    timestamp: u64,
}

public struct DataProcessed has copy, drop {
    submission_id: ID,
    original_blob_id: vector<u8>,
    processed_blob_id: vector<u8>,
    processor_enclave: ID,
    timestamp: u64,
}

// Initialize function - create enclave capability and config
fun init(otw: TELEGRAM, ctx: &mut TxContext) {
    let cap = enclave::new_cap(otw, ctx);

    // Create enclave config with placeholder PCRs (to be updated later)
    cap.create_enclave_config(
        b"telegram processing enclave".to_string(),
        x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", // pcr0
        x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", // pcr1
        x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", // pcr2
        ctx,
    );

    transfer::public_transfer(cap, ctx.sender())
}

/// Submit encrypted Telegram data for processing
/// Called by users to upload their encrypted Telegram messages
public fun submit_telegram_data(
    blob_id: vector<u8>,
    ctx: &mut TxContext,
): ID {
    let submission = TelegramSubmission {
        id: object::new(ctx),
        blob_id,
        submitter: ctx.sender(),
        timestamp: tx_context::epoch_timestamp_ms(ctx),
        processed: false,
        processing_result_blob_id: option::none(),
    };

    let submission_id = object::id(&submission);

    // Emit event
    sui::event::emit(DataSubmitted {
        submission_id,
        blob_id,
        submitter: ctx.sender(),
        timestamp: tx_context::epoch_timestamp_ms(ctx),
    });

    // Transfer to submitter for ownership
    transfer::transfer(submission, ctx.sender());
    
    submission_id
}

/// Mark data as processed by TEE
/// Called by TEE after successful processing with signed proof
public fun mark_processed<T>(
    mut submission: TelegramSubmission,
    enclave: &Enclave<T>,
    processed_blob_id: vector<u8>,
    message_count: u64,
    processing_metadata: String,
    timestamp_ms: u64,
    signature: &vector<u8>,
    ctx: &mut TxContext,
) {
    // Verify data is not already processed
    assert!(!submission.processed, EAlreadyProcessed);

    // Verify TEE signature
    let processing_response = TelegramProcessingResponse {
        original_blob_id: submission.blob_id,
        processed_blob_id,
        message_count,
        processing_metadata,
    };

    let verified = enclave.verify_signature(
        TELEGRAM_PROCESSING_INTENT,
        timestamp_ms,
        processing_response,
        signature,
    );
    assert!(verified, EInvalidSignature);

    // Update submission
    submission.processed = true;
    submission.processing_result_blob_id = option::some(processed_blob_id);

    // Emit event
    sui::event::emit(DataProcessed {
        submission_id: object::id(&submission),
        original_blob_id: submission.blob_id,
        processed_blob_id,
        processor_enclave: object::id(enclave),
        timestamp: timestamp_ms,
    });

    // Keep ownership with original submitter
    let submitter = submission.submitter;
    transfer::transfer(submission, submitter);
}

/// Get submission details (read-only)
public fun get_submission_info(submission: &TelegramSubmission): (vector<u8>, address, u64, bool) {
    (submission.blob_id, submission.submitter, submission.timestamp, submission.processed)
}

/// Get processing result blob ID if available
public fun get_processing_result(submission: &TelegramSubmission): Option<vector<u8>> {
    submission.processing_result_blob_id
}

/// Check if data has been processed
public fun is_processed(submission: &TelegramSubmission): bool {
    submission.processed
}

#[test_only]
public fun destroy_for_testing(submission: TelegramSubmission) {
    let TelegramSubmission { 
        id, 
        blob_id: _, 
        submitter: _, 
        timestamp: _, 
        processed: _, 
        processing_result_blob_id: _ 
    } = submission;
    id.delete();
}

#[test]
fun test_telegram_submission_flow() {
    use sui::test_scenario::{Self, ctx};
    use sui::test_utils::destroy;
    use enclave::enclave::{Self, EnclaveConfig};

    let mut scenario = test_scenario::begin(@0x1);
    
    // Initialize contract
    init(TELEGRAM {}, scenario.ctx());
    scenario.next_tx(@0x1);

    // Submit data
    let blob_id = b"test_blob_id_123";
    let submission_id = submit_telegram_data(blob_id, scenario.ctx());
    
    scenario.next_tx(@0x1);
    
    // Verify submission was created
    let submission = scenario.take_from_sender<TelegramSubmission>();
    let (stored_blob_id, submitter, _, processed) = get_submission_info(&submission);
    
    assert!(stored_blob_id == blob_id, 0);
    assert!(submitter == @0x1, 1);
    assert!(!processed, 2);
    
    destroy_for_testing(submission);
    scenario.end();
}