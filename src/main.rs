use alloy::providers::{ProviderBuilder, WsConnect};
use dotenvy::dotenv;
use eacc_rs::filter_publish_job_events;
use eacc_rs::telegram_api::{notification_worker, JobNotification};
use eacc_rs::telemetry::{get_subscriber, init_subscriber};
use eyre::{Error, Result};
use std::env;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // TODO: Create env variables
    // TODO: Write logs to files

    dotenv().ok(); // Loads variables from .env into the process

    let rpc_api = env::var("RPC_API").expect("RPC_API not set");
    let tg_api = env::var("TELEGRAM_BOT_API").expect("TELEGRAM_BOT_API not set");
    let tg_chat = env::var("TG_CHAT_ID").expect("TG_CHAT_ID not set");

    // REDIRECT ALL 'LOG'S EVENTS TO OUR SUBSCRIVER
    let subscriber = get_subscriber("eacc_rs".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    tracing::info!("RPC API -> {}!", rpc_api);

    // Create ws provider
    let ws = WsConnect::new(format!(
        "wss://arbitrum-mainnet.infura.io/ws/v3/{}",
        rpc_api
    ));
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    // Create mpsc queue
    let (tx, rx) = mpsc::channel::<JobNotification>(100);

    // Spawn notification worker task
    tokio::spawn(notification_worker(rx, tg_api, tg_chat));

    // Spawn event fetching task
    tokio::spawn(filter_publish_job_events(provider, tx));

    // TODO: Enhance below logic for shut down and reruns
    // Wait for Ctrl+C to exit
    tokio::signal::ctrl_c().await?;
    tracing::info!("Received Ctrl+C, shutting down");
    Ok(())
}
