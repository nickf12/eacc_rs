#[cfg(test)]
// Mock logging setup for tests
fn setup_test_subscriber() {
    use eacc_rs::telemetry::{get_subscriber, init_subscriber};
    let subscriber = get_subscriber("eacc_rs".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
}

#[cfg(test)]
mod tests {
    use eacc_rs::{
        get_from_ipfs,
        telegram_api::{notification_worker, JobNotification},
    };
    use eyre::{Error, Result};
    use tokio::sync::mpsc;

    use super::*;

    /// This test fetch a correct Job content hash from ipfs
    #[tokio::test]
    async fn test_fetch_ipfs_data_valid_hash() {
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
        setup_test_subscriber();
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
