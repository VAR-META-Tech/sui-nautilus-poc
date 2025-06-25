#!/usr/bin/env -S cargo +nightly -Zscript
// ```cargo
// [dependencies]
// tokio = { version = "1.0", features = ["full"] }
// reqwest = { version = "0.11", features = ["json"] }
// serde_json = "1.0"
// anyhow = "1.0"
// ```

use reqwest::Client;
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 7 {
        eprintln!("Usage: {} <address> <blob_id> <on_chain_file_obj_id> <policy_object_id> <threshold> <enclave_id>", args[0]);
        std::process::exit(1);
    }

    let address = &args[1];
    let blob_id = &args[2];
    let on_chain_file_obj_id = &args[3];
    let policy_object_id = &args[4];
    let threshold: u32 = args[5].parse()?;
    let enclave_id = &args[6];

    let client = Client::new();
    let base_url = env::var("NAUTILUS_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let payload = json!({
        "payload": {
            "address": address,
            "blob_id": blob_id,
            "on_chain_file_obj_id": on_chain_file_obj_id,
            "policy_object_id": policy_object_id,
            "threshold": threshold,
            "enclave_id": enclave_id
        }
    });

    println!("Sending request to: {}/process_seal_task", base_url);
    println!("Payload: {}", serde_json::to_string_pretty(&payload)?);

    let response = client
        .post(&format!("{}/process_seal_task", base_url))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    let response_text = response.text().await?;

    println!("Response Status: {}", status);
    println!("Response Body: {}", response_text);

    if status.is_success() {
        if let Ok(json_response) = serde_json::from_str::<serde_json::Value>(&response_text) {
            println!("Formatted Response: {}", serde_json::to_string_pretty(&json_response)?);
        }
    }

    Ok(())
} 