use std::env;
use twitter_api_v1::endpoints::tweets::manage_tweets::create_tweet;
use twitter_api_v1::endpoints::EndpointRet;
use twitter_api_v1::TokenSecrets;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok(); // Load environment variables
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

    let ret = create_tweet(
        &token_secrets,
        client,
        Some("Hello world, this is my first post on X from a Rust application!"),
        None,
        None,
    )
    .await?;
    match ret {
        EndpointRet::Ok(ok_json) => {
            println!("create_tweet:{ok_json:?}");
        }
        _x => panic!("{_x:?}"),
    };

    Ok(())
}
