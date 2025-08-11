use std::env;
use std::fs::File;
use std::io::Read;

use reqwest::Body;
use std::path::Path;

use crate::{error::AppError, JobNotification};
use reqwest::Client;
use tokio::sync::mpsc;
use twitter_api_v1::endpoints::tweets::manage_tweets::create_tweet;
use twitter_api_v1::endpoints::EndpointRet;
use twitter_api_v1::objects::MediaCategory;
use twitter_api_v1::TokenSecrets;

#[tracing::instrument(name = "send_x_notification", skip(client, token_secrets))]
async fn send_x_notification(
    client: Client,
    token_secrets: TokenSecrets,
    notification: &JobNotification,
    media_id: u64,
) -> Result<u64, AppError> {
    let message = format!(
        "Title: {}\nReward: {} ${}\nDetails: effectiveacceleration.ai/dashboard/jobs/{}",
        notification.title, notification.amount, notification.symbol, notification.job_id
    );

    let ret = create_tweet(
        &token_secrets,
        client,
        Some(&message),
        Some(vec![media_id]),
        None,
    )
    .await?;

    match ret {
        EndpointRet::Ok(ok_json) => {
            tracing::info!("create_tweet:{ok_json:?}");
            Ok(ok_json.id)
        }
        _x => Err(AppError::XApi(format!("{:?}", _x))),
    }
}

#[tracing::instrument(name = "x_upload_image", skip_all)]
pub async fn x_upload_image(
    token_secrets: &TokenSecrets,
    client: Client,
    media_category: twitter_api_v1::objects::MediaCategory,
    stream: reqwest::Body,
    stream_length: Option<u64>,
    file_name: Option<String>,
) -> Result<u64, AppError> {
    let ret = twitter_api_v1::endpoints::media::upload_media::upload_image(
        token_secrets,
        client,
        media_category,
        stream,
        stream_length,
        file_name,
    )
    .await?;

    match ret {
        EndpointRet::Ok(ok_json) => {
            println!("upload_media:{ok_json:?}");
            Ok(ok_json.media_id)
        }
        _x => Err(AppError::XApi(format!("{:?}", _x))),
    }
}

#[tracing::instrument(name = "x_worker", skip(rx))]
pub async fn x_worker(mut rx: mpsc::Receiver<JobNotification>) -> Result<(), AppError> {
    tracing::debug!("x_worker started, loading env variables...");
    let consumer_key = env::var("X_API_KEY").expect("X_API_KEY not found in .env file");
    tracing::debug!("Loaded X_API_KEY: {:?}", consumer_key);
    let consumer_secret =
        env::var("X_API_KEY_SECRET").expect("X_API_KEY_SECRET not found in .env file");
    tracing::debug!("Loaded X_API_KEY_SECRET: {:?}", consumer_secret);
    let access_token = env::var("X_ACCESS_TOKEN").expect("ACCESS_TOKEN not found in .env file");
    tracing::debug!("Loaded X_ACCESS_TOKEN: {:?}", access_token);
    let access_token_secret =
        env::var("X_ACCESS_TOKEN_SECRET").expect("ACCESS_TOKEN_SECRET not found in .env file");
    tracing::debug!("Loaded X_ACCESS_TOKEN_SECRET {}", access_token_secret);

    let token_secrets = TokenSecrets::new(
        consumer_key,
        consumer_secret,
        access_token,
        access_token_secret,
    );
    tracing::debug!("token secret: {}", token_secrets.consumer_key);

    let client = reqwest::Client::builder()
        .connection_verbose(env::var("RUST_LOG").map(|x| x.starts_with("trace")) == Ok(true))
        .danger_accept_invalid_certs(true)
        .build()?;
    tracing::debug!("client built ");

    // Upload image to X and fetch the media ID
    // Media ID are persistent, so no need to re-upload it every time
    // Path to the image file
    // TODO: Update media file
    let image_path = "./media/tweet_img.png";

    let mut file = File::open(image_path)
        .map_err(|e| AppError::XApi(format!("Can't find image, error: {e}")))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("Failed to read image file");

    // Get the file size for stream_length
    let file_size = file
        .metadata()
        .map_err(|e| AppError::XApi(format!("Error checking the file size: {e}")))?
        .len();

    let media_category = MediaCategory::TweetImage;
    let stream = Body::from(buffer);
    let file_name = Some(
        Path::new(image_path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    );
    let media_id = x_upload_image(
        &token_secrets,
        client.clone(),
        media_category,
        stream,
        Some(file_size),
        file_name,
    )
    .await?;
    tracing::debug!("waiting jobs ");

    while let Some(notification) = rx.recv().await {
        if let Err(e) = send_x_notification(
            client.clone(),
            token_secrets.clone(),
            &notification,
            media_id,
        )
        .await
        {
            tracing::error!("Failed to send X notification: {}", e);
        }
    }
    // TODO: Update return with post_id when the backend/DB is ready
    Ok(())
}
