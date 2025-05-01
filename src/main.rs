use alloy::providers::{ProviderBuilder, WsConnect};
use eacc_rs::filter_publish_job_events;
use eacc_rs::telegram_api::{notification_worker, JobNotification};
use eacc_rs::telemetry::{get_subscriber, init_subscriber};
use eyre::{Error, Result};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // TODO: Create env variables
    // TODO: Write logs to files
    // REDIRECT ALL 'LOG'S EVENTS TO OUR SUBSCRIVER
    let subscriber = get_subscriber("eacc_rs".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    tracing::info!("Hello world!");

    // Create ws provider
    let ws =
        WsConnect::new("wss://arbitrum-mainnet.infura.io/ws/v3/a4921b52d671477e8622579c84ecf959");
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    // Create mpsc queue
    let (tx, rx) = mpsc::channel::<JobNotification>(100);

    // Spawn notification worker task
    tokio::spawn(notification_worker(
        rx,
        "7675454109:AAHWGRpyKT_I8mNpFQIsQ9q45lD_T7Hg0pw".to_string(),
        "5967208142".to_string(), // ""telegram_chat_id.to_string(),
                                  // twitter_credentials.clone(),
    ));

    // Spawn event fetching task
    tokio::spawn(filter_publish_job_events(provider, tx));

    // TODO: Enhance below logic for shut down and reruns
    // Wait for Ctrl+C to exit
    tokio::signal::ctrl_c().await?;
    tracing::info!("Received Ctrl+C, shutting down");
    Ok(())
}
