// use actix_web::{HttpRequest, HttpResponse, Responder};
use alloy::primitives::utils::format_units;
use alloy::{consensus::Transaction, primitives::address, providers::Provider, sol};
use eyre::Result;
use futures::stream::StreamExt;
use serde::Serialize;
use tokio::sync::mpsc;
pub mod error;
pub mod telegram_api;
pub mod telemetry;
pub mod utils;
pub mod x_api;
use utils::get_from_ipfs;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug)]
    MarketPlaceData,
    "./src/abis/MarketplaceData.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug)]
    IERC20,
    "./src/abis/IERC20.json"
);

// Job notification struct
#[derive(Debug, Serialize, Clone)]
pub struct JobNotification {
    pub job_id: String,
    pub title: String,
    pub description: String,
    pub amount: f64,
    pub symbol: String,
}

// Filter for PublishJobEvents
#[tracing::instrument(name = "filter_publish_job_events", skip(provider))]
pub async fn filter_publish_job_events(
    provider: impl Provider + Clone,
    queue_sender: mpsc::Sender<JobNotification>,
) -> Result<()> {
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
                                tracing::debug!("    - Job Title: {}", job.title);
                                // TODO: Fix amount conversion!
                                tracing::debug!(
                                    "    - Job Amount: {} ${}",
                                    decimal_amount,
                                    token_symbol
                                );
                                tracing::debug!("    - Job deliveryMethod: {}", job.deliveryMethod);
                                tracing::debug!("    - Job contentHash: {}", job.contentHash);
                                // Get content from IPFS
                                // Call the function
                                let job_description =
                                    match get_from_ipfs(&job.contentHash.to_string(), "").await {
                                        Ok(data) => {
                                            tracing::debug!("    - Job Description: {}", data);
                                            data
                                        }
                                        Err(e) => {
                                            return Err(eyre::eyre!(
                                                "Failed to fetch job description from IPFS: {}",
                                                e
                                            ));
                                        }
                                    };

                                let notification = JobNotification {
                                    job_id: event.jobId.to_string(),
                                    title: job.title,
                                    description: job_description,
                                    amount: decimal_amount,
                                    symbol: token_symbol,
                                };
                                match queue_sender.send(notification).await {
                                    Ok(_) => {
                                        tracing::debug!("    - Notification sent to the queue");
                                    }
                                    Err(e) => tracing::error!(
                                        "    - Error returned sending notification into the queue is: {}",
                                        e
                                    ),
                                }
                                tracing::debug!("    - Notification sent to the queue");
                            }
                            _ => {
                                tracing::debug!("    - Handling other function...");
                            }
                        }
                    }
                    Err(e) => tracing::error!("    - Error in stream: {:?}", e),
                }
            }
        }
        Err(e) => {
            tracing::error!("Error JobEvent filter = {}", e)
        }
    }
    Ok(())
}
