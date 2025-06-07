use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::sync::mpsc;

// Job notification struct
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobNotification {
    pub job_id: String,
    pub title: String,
    pub description: String,
    pub amount: f64,
    pub symbol: String,
}

// Send notification to Telegram
#[tracing::instrument(name = "send_telegram_notification", skip(client, bot_token))]
async fn send_telegram_notification(
    client: &Client,
    notification: &JobNotification,
    bot_token: &str,
    chat_id: &str,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let message = format!(
        "<b>A new job has been published in EACC</b>\n\n\n<b>Title</b>:<a href='https://staging.effectiveacceleration.ai/dashboard/jobs/{}'>{}</a>\n<b>Job Description</b>:\n{}\n\n<b>Job Reward</b>: {} ${}\n\n",
        notification.job_id,
        notification.title,
        notification.description,
        notification.amount,
        notification.symbol
    );
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let response = client
        .post(&url)
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": message,
            "parse_mode": "html"
        }))
        .send()
        .await?;

    if response.status().is_success() {
        tracing::info!("Sent Telegram notification for job {}", notification.job_id);
        Ok(())
    } else {
        let resp_status = response.status();
        let error_text = response.text().await.unwrap_or_default();

        Err(format!(
            "Telegram API error: status: {}, text: {}",
            resp_status, error_text
        )
        .into())
    }
}

// Notification worker
#[tracing::instrument(
    name = "notification_worker",
    skip(rx, telegram_bot_token, telegram_chat_id)
)]
pub async fn notification_worker(
    mut rx: mpsc::Receiver<JobNotification>,
    telegram_bot_token: String,
    telegram_chat_id: String,
    // twitter_credentials: TwitterCredentials,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let client = Client::new();
    while let Some(notification) = rx.recv().await {
        // Send to Telegram
        if let Err(e) = send_telegram_notification(
            &client,
            &notification,
            &telegram_bot_token,
            &telegram_chat_id,
        )
        .await
        {
            tracing::error!("Failed to send Telegram notification: {}", e);
        }
        // TODO: ADD twitter and other socials to push notifications
        // Send to Twitter
        // if let Err(e) =
        //     send_twitter_notification(&client, &notification, &twitter_credentials).await
        // {
        //     eprintln!("Failed to send Twitter notification: {}", e);
        // }
    }
    Ok(())
}
