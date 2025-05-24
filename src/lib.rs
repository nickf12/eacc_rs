use alloy::primitives::utils::format_units;
use alloy::{
    consensus::Transaction,
    hex::{self},
    primitives::address,
    providers::Provider,
    sol,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use cid::multihash::Multihash;
use cid::Cid;
use eyre::Result;
use futures::stream::StreamExt;
use reqwest::ClientBuilder;
use std::env;
use std::{error::Error, time::Duration};
use telegram_api::JobNotification;
use tokio::sync::mpsc;

pub mod telegram_api;
pub mod telemetry;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug)]
    MarketPlaceData,
    "./src/abis/MarketplaceData.json"
);

// Not needed for the moment.
// sol!(
//     #[allow(missing_docs)]
//     #[sol(rpc)]
//     #[derive(Debug)]
//     MarketPlace,
//     "./src/abis/Marketplace.json"
// );

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug)]
    IERC20,
    "./src/abis/IERC20.json"
);

// Healt check
#[tracing::instrument(name = "health_check")]
pub async fn health_check() -> &'static str {
    "OK"
}

// Placeholder decryption for UTF-8 data
fn decrypt_utf8_data(
    data: &[u8],
    _session_key: &str,
) -> Result<String, Box<dyn Error + Send + Sync + 'static>> {
    // Input: decrypted Base64 data (encrypted bytes)
    // Output: UTF-8 string (job description)
    String::from_utf8(data.to_vec()).map_err(|e| format!("Invalid UTF-8: {}", e).into())
}

// Replicate hashToCid
fn hash_to_cid(hash: &str) -> Result<String, Box<dyn Error + Send + Sync + 'static>> {
    let hash_bytes = hex::decode(hash)?;
    if hash_bytes.len() != 32 {
        return Err("Invalid hash: must be 32 bytes".into());
    }
    let multihash = Multihash::<64>::wrap(0x12, &hash_bytes)?;
    let cid = Cid::new_v0(multihash)?;
    Ok(cid.to_string()) // Returns Base58-encoded CIDv0
}

// Fetch raw Base64-encoded encrypted data from IPFS
#[tracing::instrument(name = "get_from_ipfs_raw", skip(_session_key))]
async fn get_from_ipfs_raw(
    content_hash: &str,
    _session_key: &str,
) -> Result<String, Box<dyn Error + Send + Sync + 'static>> {
    // Convert to CID
    let cid_str = if content_hash.starts_with("Qm") {
        content_hash
    } else {
        &hash_to_cid(content_hash)?
    };

    let cid = Cid::try_from(cid_str)?;
    tracing::debug!("Generated CID: {}", cid);

    // Create reqwest client with timeout
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()?;

    // Use local IPFS gateway
    let gateway =
        env::var("IPFS_GATEWAY").expect("IPFS Gateway not set in the environment variables");
    let url = format!("{}{}", gateway, cid);

    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let data = response.text().await?;
        // Validate Base64
        BASE64
            .decode(&data)
            .map_err(|e| format!("Response is not valid Base64: {}", e))?;
        tracing::debug!("Fetched from: {}", url);
        Ok(data)
    } else {
        Err(format!("Failed to fetch from {}: {}", url, response.status()).into())
    }
}

// Fetch and decrypt IPFS data to UTF-8
#[tracing::instrument(name = "get_from_ipfs", skip(session_key))]
pub async fn get_from_ipfs(
    content_hash: &str,
    session_key: &str,
) -> Result<String, Box<dyn Error + Send + Sync + 'static>> {
    // Fetch raw Base64-encoded data
    let base64_data = get_from_ipfs_raw(content_hash, session_key).await?;

    // Decode Base64
    let decoded_data = BASE64
        .decode(&base64_data)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    // Decrypt to UTF-8
    let decrypted_data = decrypt_utf8_data(&decoded_data, session_key)?;

    Ok(decrypted_data)
}

// TODO: Improve timeout/error handling/api
#[tracing::instrument(name = "filter_publish_job_events", skip(provider))]
pub async fn filter_publish_job_events(
    provider: impl Provider + Clone,
    queue_sender: mpsc::Sender<JobNotification>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let marketplace_data = MarketPlaceData::new(
        address!("0191ae69d05F11C7978cCCa2DE15653BaB509d9a"),
        provider.clone(),
    );

    let filter = marketplace_data.JobEvent_filter().from_block(278858754);

    match filter.subscribe().await {
        Ok(subscription) => {
            let mut event_stream = subscription.into_stream();

            while let Some(log) = event_stream.next().await {
                match log {
                    Ok((event, raw_log)) => {
                        // Found a new JobEvent -> evaluate who called it

                        let tx_hash = raw_log
                            .transaction_hash
                            .ok_or("No transaction hash in log")
                            .unwrap();
                        tracing::info!("Tx hash -> {tx_hash}");

                        let tx = provider
                            .get_transaction_by_hash(tx_hash)
                            .await?
                            .ok_or("Transaction not found")
                            .unwrap();

                        let input_data = tx.input();
                        let function_selector = &input_data[..4];
                        // Map MethodIDs to function signatures
                        let function_signature = match function_selector {
                            [0x3a, 0x08, 0x08, 0x39] => {
                                "publishJobEvent(uint256,(uint8,bytes,bytes,uint32))".to_string()
                            }
                            _ => format!("Unknown function (selector: {:x?})", function_selector),
                        };

                        tracing::debug!("Event Details:");
                        tracing::debug!("- Job ID: {:?}", event.jobId); // Assuming jobId exists in your event

                        tracing::debug!("- Emitting Function: {}", function_signature);
                        tracing::debug!("- Tx Hash: {:?}", tx_hash);
                        tracing::debug!("- MethodID: {:x?}", function_selector);
                        let event_data = event.eventData;
                        //let data = event_data.data_;
                        // Example condition based on the function
                        match function_signature.as_str() {
                            "publishJobEvent(uint256,(uint8,bytes,bytes,uint32))" => {
                                tracing::info!("Handling publishJobEvent...");
                                // Access event data
                                tracing::debug!("Event Data: {:?}", event_data);
                                // Get The JobPost data
                                let job = marketplace_data.getJob(event.jobId).call().await?._0;
                                let token_contract = IERC20::new(job.token, provider.clone());

                                // Use multicall when possible to reduce amount of requests to public RPC
                                let multicall = provider
                                    .multicall()
                                    .add(token_contract.symbol())
                                    .add(token_contract.decimals());
                                let (token_symbol, token_decimals) = multicall.aggregate().await?;
                                let token_symbol = token_symbol._0;

                                let token_decimals = token_decimals._0;
                                let formatted_amount = format_units(job.amount, token_decimals)?;
                                let decimal_amount: f64 = formatted_amount.parse().unwrap();
                                tracing::info!("    - Job Title: {}", job.title);
                                // TODO: Fix amount conversion!
                                tracing::info!(
                                    "    - Job Amount: {} ${}",
                                    decimal_amount,
                                    token_symbol
                                );
                                tracing::debug!("    - Job deliveryMethod: {}", job.deliveryMethod);
                                tracing::debug!("    - Job contentHash: {}", job.contentHash);
                                // Get content from IPFS
                                // Call the function
                                let result = get_from_ipfs(&job.contentHash.to_string(), "").await;
                                let mut job_description = "".to_string();
                                // Check the result
                                match result {
                                    Ok(data) => {
                                        // If data is returned, ensure it's not empty

                                        tracing::debug!("    - Job Description: {}", data);
                                        job_description = data;
                                    }
                                    Err(e) => {
                                        // If the IPFS data isn't available, the error is likely a 404 or similar
                                        // We don't fail the test for network issues, but log the error
                                        tracing::error!(
                                            "Expected error (e.g., data not found on IPFS): {}",
                                            e
                                        );
                                    }
                                }
                                let notification = JobNotification {
                                    job_id: event.jobId.to_string(),
                                    title: job.title,
                                    description: job_description,
                                    amount: decimal_amount,
                                    symbol: token_symbol,
                                };
                                match queue_sender.send(notification).await {
                                    Ok(_) => {
                                        tracing::info!("Notification stored into the queue");
                                    }
                                    Err(e) => tracing::error!(
                                        "Error returned storing notification into the queue is: {}",
                                        e
                                    ),
                                }
                                tracing::info!("    - Notification sent to the queue");

                                // let cid = hash_to_cid(&job.contentHash.to_string()).unwrap();
                                // //let data = get_from_ipfs_raw(&cid).await.unwrap();
                                // println!("    - Data -> {cid}");
                            }
                            _ => {
                                tracing::info!("Handling other function...");
                                // Add your logic for other functions
                            }
                        }
                    }
                    Err(e) => tracing::error!("Error in stream: {:?}", e),
                }
            }
        }
        Err(e) => {
            tracing::error!("Error JobEvent filter = {}", e)
        }
    }
    Ok(())
}
