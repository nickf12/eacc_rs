use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Environment variable error: {0}")]
    EnvVar(#[from] std::env::VarError),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("X API error: {0}")]
    XApi(String),

    #[error("Telegram API error: {0}")]
    TelegramApi(String),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Event parsing error: {0}")]
    EventParsing(String),

    #[error("Reqwest error: {0}")]
    Reqwest(#[from] twitter_api_v1::endpoints::EndpointError),
}
