// use actix_web::{HttpRequest, HttpResponse, Responder};
use alloy::primitives::utils::format_units;
use alloy::{consensus::Transaction, primitives::address, providers::Provider, sol};
use eyre::Result;
use futures::stream::StreamExt;
use serde::Serialize;
use tokio::sync::mpsc;
use utils::get_from_ipfs;

pub mod error;
pub mod telegram_api;
pub mod telemetry;
pub mod utils;
pub mod x_api;

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
    MarketPlace,
    "./src/abis/Marketplace.json"
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

// Filter EACCRewardsDistributed events
#[tracing::instrument(name = "filter_eacc_rewards_distributed", skip(provider))]
pub async fn filter_eacc_rewards_distributed(
    provider: impl Provider + Clone,
    jobs_completed_queue: mpsc::Sender<JobNotification>,
) -> Result<()> {
    let marketplace = MarketPlace::new(
        address!("0x405AcFbD1400A168fDd4aDA2D214e8Ae5FF7a624"),
        provider.clone(),
    );

    let eacc_distribution_filter = marketplace.EACCRewardsDistributed_filter();

    match eacc_distribution_filter.subscribe().await {
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

                        // Transaction block number and timestamp
                        let block_number = raw_log.block_number.unwrap_or_default();
                        let block_timestamp = raw_log.block_timestamp.unwrap_or_default();

                        tracing::info!("Tx hash -> {tx_hash}, Block Number: {block_number}, Timestamp: {block_timestamp}");

                        // let tx = provider
                        //     .get_transaction_by_hash(tx_hash)
                        //     .await?
                        //     .ok_or("Transaction not found")
                        //     .unwrap();

                        // let input_data = tx.input();

                        let job_id = event.jobId; // Assuming jobId exists in your event
                        let job = marketplace.getJob(job_id).call().await?._0; // Get job details

                        let token_contract = IERC20::new(job.token, provider.clone());

                        // Use multicall when possible to reduce amount of requests to public RPC
                        let multicall = provider
                            .multicall()
                            .add(token_contract.symbol())
                            .add(token_contract.decimals())
                            .add(token_contract.name());

                        // Fetch token details
                        let (token_symbol, token_decimals, token_name) =
                            multicall.aggregate().await?;

                        let token_symbol = token_symbol._0;

                        let token_decimals = token_decimals._0;
                        let token_name = token_name._0;
                        let formatted_amount: String = format_units(job.amount, token_decimals)?;
                        let _decimal_amount: f64 = formatted_amount.parse().unwrap();

                        // get rewards
                        let reward_amount = event.rewardAmount;
                        let formatted_reward_amount: String =
                            format_units(reward_amount, token_decimals)?;

                        // Calculate protocol reward in USD
                        let usd_price = match fetch_token_usd_price(&job.token.to_string()).await {
                            Ok(price) => price,
                            Err(e) => {
                                tracing::error!(
                                    "Error fetching USD price for {}: {}",
                                    job.token,
                                    e
                                );
                                // Default to 1.1 if error occurs
                                1.1
                            }
                        };
                        // Calculate percentage fee taken by the protocol 6.96%
                        let protocol_fee_percentage = 0.0696;
                        let reward_amount_f64: f64 = formatted_reward_amount.parse().unwrap();
                        let reward_usd_value =
                            reward_amount_f64 * usd_price * protocol_fee_percentage;

                        tracing::info!(
                            "Job_id: {}, Token: {}, Reward Amount: {}, Protocol Fee (6.96%): ${}",
                            job_id,
                            token_name,
                            formatted_reward_amount,
                            reward_usd_value
                        );

                        tracing::info!(
                            "Job_id: {}, Token: {}, Amount: {}, Reward: {}",
                            job_id,
                            token_name,
                            formatted_amount,
                            formatted_reward_amount
                        );
                        // let decimal_reward_amount: f64 = formatted_reward_amount.parse().unwrap();

                        // Build notification and send it to the socials

                        // Create Notification
                        let notification = JobNotification {
                            job_id: event.jobId.to_string(),
                            title: job.title,
                            description: "".to_string(),
                            amount: reward_amount_f64,
                            symbol: token_symbol,
                        };
                    }
                    Err(e) => tracing::error!("    - Error in stream: {:?}", e),
                }
            }
        }
        Err(e) => {
            tracing::error!("RPCError with EACCDist event filter = {}", e)
        }
    }
    unimplemented!();
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

    let filter = marketplace_data.JobEvent_filter();

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
                                tracing::info!("Event Data: {:?}", event_data);
                                // Get The JobPost data
                                let job = marketplace_data.getJob(event.jobId).call().await?._0;
                                let token_contract = IERC20::new(job.token, provider.clone());

                                // Use multicall when possible to reduce amount of requests to public RPC
                                let multicall = provider
                                    .multicall()
                                    .add(token_contract.symbol())
                                    .add(token_contract.decimals())
                                    .add(token_contract.name());

                                let (token_symbol, token_decimals, token_name) =
                                    multicall.aggregate().await?;
                                let mut token_symbol = token_symbol._0;

                                let token_decimals = token_decimals._0;
                                let mut token_name = token_name._0;
                                let formatted_amount: String =
                                    format_units(job.amount, token_decimals)?;
                                let decimal_amount: f64 = formatted_amount.parse().unwrap();

                                // Fetch USD price for the token
                                let usd_price =
                                    match fetch_token_usd_price(&job.token.to_string()).await {
                                        Ok(price) => price,
                                        Err(e) => {
                                            tracing::error!(
                                                "Error fetching USD price for {}: {}",
                                                job.token,
                                                e
                                            );
                                            // Default to 1.0 if error occurs
                                            1.1
                                        }
                                    };
                                tracing::info!(
                                    "Job_id: {}, Token: {}, USD Price: {}",
                                    event.jobId,
                                    token_name,
                                    usd_price
                                );

                                let dollar_value = decimal_amount * usd_price;

                                tracing::info!(
                                    "Job_id: {}, Token: {}, Amount: {}, USD Value: ${}",
                                    event.jobId,
                                    token_name,
                                    formatted_amount,
                                    dollar_value
                                );
                                if dollar_value < 0.1 {
                                    tracing::info!(
                                        "Skipping job: value below $0.1 (value: ${})",
                                        dollar_value
                                    );
                                    continue;
                                }
                                let rounded_decimal_amount =
                                    (decimal_amount * 10_000.0).round() / 10_000.0;

                                // Filter-out not needed notifications:
                                // - reward amount < 0.01
                                // - Keywords on Title
                                // - Keywords on Description

                                tracing::debug!("    - Job Title: {}", job.title);

                                tracing::debug!(
                                    "    - Job Amount: {} ${}",
                                    rounded_decimal_amount,
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
                                            tracing::debug!(
                                                "Failed to fetch job description from IPFS: {}",
                                                e
                                            );
                                            continue;
                                        }
                                    };

                                // Create the notification
                                let notification = JobNotification {
                                    job_id: event.jobId.to_string(),
                                    title: job.title,
                                    description: job_description,
                                    amount: rounded_decimal_amount,
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
            tracing::info!("Event stream ended.");
        }
        Err(e) => {
            tracing::error!("Error JobEvent filter = {}", e)
        }
    }
    Ok(())
}

/// Fetch the USD price of a token using CoinGecko API
#[tracing::instrument(name = "fetch_token_usd_price")]
pub async fn fetch_token_usd_price(contract_address: &str) -> eyre::Result<f64> {
    let url =
        format!(" https://api.coingecko.com/api/v3/simple/token_price/arbitrum-one?contract_addresses={contract_address}&vs_currencies=usd");

    let resp = reqwest::get(&url).await;
    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                "Failed to fetch price from CoinGecko: {}, for address: {}",
                e,
                contract_address
            );
            return Err(eyre::eyre!(
                "Network error fetching price: {} for address:{}",
                e,
                contract_address
            ));
        }
    };
    let json: serde_json::Value = match resp.json().await {
        Ok(j) => j,
        Err(e) => {
            tracing::warn!("Failed to parse CoinGecko response: {}", e);
            return Err(eyre::eyre!("Failed to parse CoinGecko response: {}", e));
        }
    };
    tracing::info!(
        "CoinGecko response: {}",
        json[contract_address.to_lowercase()]
    );

    let price = json[contract_address.to_lowercase()]["usd"].as_f64();
    match price {
        Some(p) => Ok(p),
        None => {
            tracing::warn!("Price not found for token_id: {}", contract_address);
            Err(eyre::eyre!(
                "Price not found for token_id: {}",
                contract_address
            ))
        }
    }
}
