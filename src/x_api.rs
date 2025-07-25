use std::env;

use crate::{error::AppError, JobNotification};
use reqwest::Client;
use tokio::sync::mpsc;
use twitter_api_v1::endpoints::tweets::manage_tweets::create_tweet;
use twitter_api_v1::endpoints::EndpointRet;
use twitter_api_v1::TokenSecrets;

#[tracing::instrument(name = "send_x_notification", skip(client, token_secrets))]
async fn send_x_notification(
    client: Client,
    token_secrets: TokenSecrets,
    notification: &JobNotification,
) -> Result<(), AppError> {
    let message = format!(
        "New job on EACC!\nTitle: {}\nReward: {} {}\nDetails: https://staging.effectiveacceleration.ai/dashboard/jobs/{}",
        notification.title,
        notification.amount,
        notification.symbol,
        notification.job_id
    );
    let ret = create_tweet(&token_secrets, client, Some(&message), None, None).await?;

    match ret {
        EndpointRet::Ok(ok_json) => {
            println!("create_tweet:{ok_json:?}");
        }
        _x => tracing::error!("Error creating tweet: {_x:?}"),
    };
    Ok(())
}

#[tracing::instrument(name = "x_worker", skip(rx))]
pub async fn x_worker(mut rx: mpsc::Receiver<JobNotification>) -> Result<(), AppError> {
    let consumer_key = env::var("X_API_KEY").expect("X_API_KEY not found in .env file");
    let consumer_secret =
        env::var("X_API_KEY_SECRET").expect("X_API_KEY_SECRET not found in .env file");
    let access_token = env::var("X_ACCESS_TOKEN").expect("ACCESS_TOKEN not found in .env file");
    let access_token_secret =
        env::var("X_ACCESS_TOKEN_SECRET").expect("ACCESS_TOKEN_SECRET not found in .env file");

    let token_secrets = TokenSecrets::new(
        consumer_key,
        consumer_secret,
        access_token,
        access_token_secret,
    );
    let client = reqwest::Client::builder()
        .connection_verbose(env::var("RUST_LOG").map(|x| x.starts_with("trace")) == Ok(true))
        .danger_accept_invalid_certs(true)
        .build()?;

    while let Some(notification) = rx.recv().await {
        if let Err(e) =
            send_x_notification(client.clone(), token_secrets.clone(), &notification).await
        {
            tracing::error!("Failed to send X notification: {}", e);
        }
    }
    Ok(())
}
