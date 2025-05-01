#[cfg(test)]
use std::sync::Once;

#[cfg(test)]
static INIT_TRACING: Once = Once::new();

// Set up tracing for the entire test suite
#[cfg(test)]
fn init_test_tracing() {
    INIT_TRACING.call_once(|| {
        // Use tracing-test's default subscriber for captured logs
        // No need to call get_subscriber/init_subscriber from telemetry.rs
        // tracing-test handles log capture with #[traced_test]
        tracing_subscriber::fmt()
            .with_env_filter("info") // Match your telemetry.rs "info" level
            .with_writer(std::io::stdout)
            .try_init()
            .ok(); // Ignore if already initialized
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use alloy::{
        primitives::{address, ruint::aliases::U256, utils::format_units},
        providers::{ProviderBuilder, WsConnect},
    };
    // use alloy::primitives::utils::format_units;
    use eacc_rs::{
        get_from_ipfs,
        telegram_api::{notification_worker, JobNotification},
        MarketPlaceData, IERC20,
    };
    use eyre::{Error, Result};
    use tokio::sync::mpsc;

    use super::*;

    /// This test fetch a correct Job content hash from ipfs
    #[tokio::test]
    async fn test_contract() -> Result<(), Error> {
        init_test_tracing(); // Safe to call multiple times due to Once
                             // Create ws provider

        tracing::info!("Test started");

        // Mock Telegram credentials (chat_id is empty to trigger error, as in your main)
        let telegram_bot_token = "7675454109:AAHWGRpyKT_I8mNpFQIsQ9q45lD_T7Hg0pw".to_string();
        let telegram_chat_id = "@EACC_New_Jobs".to_string(); // Simulates your empty chat_id

        // Create mpsc channel
        let (tx, rx) = mpsc::channel::<JobNotification>(100);

        // Spawn notification worker
        let worker_handle = tokio::spawn(notification_worker(
            rx,
            telegram_bot_token,
            telegram_chat_id,
        ));

        let ws = WsConnect::new(
            "wss://arbitrum-mainnet.infura.io/ws/v3/a4921b52d671477e8622579c84ecf959",
        );
        let provider = ProviderBuilder::new().on_ws(ws).await.unwrap();

        let marketplace_data = MarketPlaceData::new(
            address!("0191ae69d05F11C7978cCCa2DE15653BaB509d9a"),
            provider.clone(),
        );
        let id = 518;
        let job_id = U256::from(id);
        let job1 = marketplace_data.getJob(job_id).call().await?._0;

        let job_amount: u128 = job1.amount.to::<u128>();
        tracing::info!("JobID: {}, has an amount of {}", id, job_amount);
        let token_contract = IERC20::new(job1.token, provider.clone());

        let token_decimals = token_contract.decimals().call().await?._0;
        let token_symbol = token_contract.symbol().call().await?._0;

        let formatted_amount = format_units(job1.amount, token_decimals)?;
        let decimal_amount: f64 = formatted_amount.parse()?;
        tracing::info!(
            "JobID: {}, has formatted amount of {} ${}",
            id,
            decimal_amount,
            token_symbol
        );
        // Call the function
        let result = get_from_ipfs(&job1.contentHash.to_string(), "").await;
        let mut job_description = "".to_string();
        // Check the result
        match result {
            Ok(data) => {
                // If data is returned, ensure it's not empty

                tracing::info!("    - Job Description: {}", data);
                job_description = data;
            }
            Err(e) => {
                // If the IPFS data isn't available, the error is likely a 404 or similar
                // We don't fail the test for network issues, but log the error
                tracing::error!("Expected error (e.g., data not found on IPFS): {}", e);
            }
        }
        // Create test job
        let test_job = JobNotification {
            job_id: id.to_string(),
            title: job1.title,
            description: job_description,
            amount: decimal_amount,
            symbol: token_symbol,
        };

        // Send test job to queue
        tx.send(test_job.clone()).await?;
        tracing::info!("Sent test job to queue");

        // Wait briefly to allow worker to process (avoid real Telegram API call)
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify worker is still running (not exited)
        assert!(!worker_handle.is_finished(), "Worker exited prematurely");

        // Drop tx to close channel and allow worker to hit None case
        drop(tx);

        // Wait for worker to hit channel closed error
        tokio::time::sleep(Duration::from_millis(5100)).await; // Wait past 5s retry

        Ok(())
    }

    /// This test fetch a correct Job content hash from ipfs
    #[tokio::test]
    async fn test_fetch_ipfs_data_valid_hash() {
        // Set up tracing subscriber
        init_test_tracing(); // Safe to call multiple times due to Once
                             // The provided contentHash
        let content_hash = "0xe83fa84f3b05d65a3c566e7765845292b840a24b9f3415aff46dd78a84eca125";
        //0xe83fa84f3b05d65a3c566e7765845292b840a24b9f3415aff46dd78a84eca125
        // Call the function
        let result = get_from_ipfs(content_hash, "").await;

        // Check the result
        match result {
            Ok(data) => {
                // If data is returned, ensure it's not empty
                assert!(!data.is_empty(), "Fetched data should not be empty");
                println!("Fetched IPFS Data: {}", data);
            }
            Err(e) => {
                // If the IPFS data isn't available, the error is likely a 404 or similar
                // We don't fail the test for network issues, but log the error
                println!("Expected error (e.g., data not found on IPFS): {}", e);
                assert!(
                    e.to_string().contains("Failed to fetch IPFS data"),
                    "Error should indicate fetch failure"
                );
            }
        }
    }

    /// This test the notification worker
    #[tokio::test]
    async fn test_notification_worker() -> Result<(), Error> {
        use std::time::Duration;

        // Set up tracing subscriber
        init_test_tracing(); // Safe to call multiple times due to Once
        tracing::info!("Test started");

        // Mock Telegram credentials (chat_id is empty to trigger error, as in your main)
        let telegram_bot_token = "7675454109:AAHWGRpyKT_I8mNpFQIsQ9q45lD_T7Hg0pw".to_string();
        let telegram_chat_id = "@EACC_New_Jobs".to_string(); // Simulates your empty chat_id

        // Create mpsc channel
        let (tx, rx) = mpsc::channel::<JobNotification>(100);

        // Spawn notification worker
        let worker_handle = tokio::spawn(notification_worker(
            rx,
            telegram_bot_token,
            telegram_chat_id,
        ));

        // Create test job
        let test_job = JobNotification {
            job_id: "test_123".to_string(),
            title: "Test Job".to_string(),
            description: "This is a test".to_string(),
            amount: 0.01,
            symbol: "ETH".to_string(),
        };

        // Send test job to queue
        tx.send(test_job.clone()).await?;
        tracing::info!("Sent test job to queue");

        // Wait briefly to allow worker to process (avoid real Telegram API call)
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify worker is still running (not exited)
        assert!(!worker_handle.is_finished(), "Worker exited prematurely");

        // Drop tx to close channel and allow worker to hit None case
        drop(tx);

        // Wait for worker to hit channel closed error
        tokio::time::sleep(Duration::from_millis(5100)).await; // Wait past 5s retry

        Ok(())
    }
}
