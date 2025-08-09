use alloy::providers::{ProviderBuilder, WsConnect};
use dotenvy::dotenv;
use eacc_rs::telegram_api::telegram_worker;
use eacc_rs::telemetry::{get_subscriber, init_subscriber};
use eacc_rs::x_api::x_worker;
use eacc_rs::{filter_publish_job_events, JobNotification};
use eyre::Result;
use std::env;
use tokio::signal;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    tracing::info!("Hello world");
    // Loads variables from .env into the process
    dotenv().ok();
    let rpc_api = env::var("RPC_API").expect("RPC_API not set");
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    // REDIRECT ALL 'LOG'S EVENTS TO OUR SUBSCRIVER
    let subscriber = get_subscriber("eacc_rs".into(), log_level, std::io::stdout);
    init_subscriber(subscriber);

    // Create ws provider
    let ws = WsConnect::new(format!(
        "wss://arbitrum-mainnet.infura.io/ws/v3/{}",
        rpc_api
    ));
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    // Create event queue
    let (event_tx, mut event_rx) = mpsc::channel::<JobNotification>(100);

    // Create platform-specific notification queues
    let (telegram_tx, telegram_rx) = mpsc::channel::<JobNotification>(100);
    let (twitter_tx, twitter_rx) = mpsc::channel::<JobNotification>(100);

    // Spawn event fetching task
    tokio::spawn(filter_publish_job_events(provider, event_tx));

    // Event dispatcher
    tokio::spawn(async move {
        while let Some(job) = event_rx.recv().await {
            let _ = telegram_tx.send(job.clone()).await;
            let _ = twitter_tx.send(job.clone()).await;
            // Add more platforms as needed
        }
    });

    // Platform-specific workers
    tokio::spawn(telegram_worker(telegram_rx));
    tokio::spawn(x_worker(twitter_rx));
    // ...
    // add more platforms as needed

    // Wait for Ctrl+C to exit
    match signal::ctrl_c().await {
        Ok(()) => {
            tracing::info!("Shut down signal received")
            // TODO: implement graceful shutdown
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
        }
    }

    Ok(())
}
