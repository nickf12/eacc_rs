use reqwest::Client;
use std::env;
use tokio::sync::mpsc;

use crate::{error::AppError, JobNotification};

// Send notification to Telegram
#[tracing::instrument(name = "send_telegram_notification", skip(client, bot_token))]
async fn send_telegram_notification(
    client: &Client,
    notification: &JobNotification,
    bot_token: &str,
    chat_id: &str,
) -> Result<(), AppError> {
    let message = format!(
        "<b>A new job has been published in EACC</b>\n\n\n<b>Title</b>:<a href='https://staging.effectiveacceleration.ai/dashboard/jobs/{}'>{}</a>\n<b>Job Description:</b>\n{}\n\n<b>Job Reward</b>: {} ${}\n\n",
        notification.job_id,
        notification.title,
        notification.description.trim(),
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

        Err(AppError::TelegramApi(format!(
            "Telegram API error: status: {}, text: {}",
            resp_status, error_text
        )))
    }
}

// Notification worker
#[tracing::instrument(name = "telegram_worker", skip(rx))]
pub async fn telegram_worker(mut rx: mpsc::Receiver<JobNotification>) -> Result<(), AppError> {
    let tg_client = Client::new();
    let telegram_bot_token = env::var("TELEGRAM_BOT_API").expect("TELEGRAM_BOT_API not set");
    let telegram_chat_id = env::var("TG_CHAT_ID").expect("TG_CHAT_ID not set");

    while let Some(notification) = rx.recv().await {
        // Send to Telegram
        if let Err(e) = send_telegram_notification(
            &tg_client,
            &notification,
            &telegram_bot_token,
            &telegram_chat_id,
        )
        .await
        {
            tracing::error!("Failed to send Telegram notification: {}", e);
        }
        tracing::info!("Notification processed in telegram: {:?}", notification);
    }
    Ok(())
}
