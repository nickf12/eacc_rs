use reqwest::Body;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use twitter_api_v1::endpoints::media::upload_media::upload_image;
use twitter_api_v1::endpoints::tweets::manage_tweets::create_tweet;
use twitter_api_v1::endpoints::EndpointRet;
use twitter_api_v1::objects::MediaCategory;
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

    // Path to the image file
    let image_path = "./media/hoodCart.jpeg";

    let mut file = File::open(image_path).map_err(|e| format!("Can't find image, error: {e}"))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("Failed to read image file");

    // Get the file size for stream_length
    let file_size = file
        .metadata()
        .map_err(|e| format!("Error checking the file size: {e}"))?
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
    // Upload the image
    let upload_result = upload_image(
        &token_secrets,
        client.clone(), // Clone the client for reuse
        media_category,
        stream,
        Some(file_size),
        file_name,
    )
    .await?;

    // Extract media_id from the response
    let media_id = match upload_result {
        EndpointRet::Ok(response_body) => response_body.media_id, // Assuming UploadResponseBodyOkJson has a media_id field
        _ => panic!(
            "Failed to upload image, unexpected response: {:?}",
            upload_result
        ),
    };

    let ret = create_tweet(
        &token_secrets,
        client,
        Some("Hello world, this is my first post on X from a Rust application!"),
        Some(vec![media_id]),
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
