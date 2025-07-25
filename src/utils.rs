use alloy::hex::{self};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use cid::multihash::Multihash;
use cid::Cid;
use eyre::Result;
use reqwest::ClientBuilder;
use std::{env, time::Duration};

// Placeholder decryption for UTF-8 data
#[tracing::instrument(name = "decrypt_utf8_data", skip(_session_key))]
fn decrypt_utf8_data(data: &[u8], _session_key: &str) -> String {
    // Input: decrypted Base64 data (encrypted bytes)
    // Output: UTF-8 string (job description)
    let res = String::from_utf8(data.to_vec())
        .map_err(|e| format!("Invalid UTF-8: {}", e))
        .unwrap_or_default();
    tracing::debug!("Decrypted data: {}", res);
    res
}

// Replicate hashToCid
#[tracing::instrument(name = "hash_to_cid")]
fn hash_to_cid(hash: &str) -> Result<String> {
    let hash_bytes = hex::decode(hash)?;
    if (hash_bytes).len() != 32 {
        return Err(eyre::eyre!("Invalid hash: must be 32 bytes"));
    }
    let multihash = Multihash::<64>::wrap(0x12, &hash_bytes)?;
    let cid = Cid::new_v0(multihash)?;
    Ok(cid.to_string())
}

// Fetch raw Base64-encoded encrypted data from IPFS
#[tracing::instrument(name = "get_from_ipfs_raw", skip(_session_key))]
async fn get_from_ipfs_raw(content_hash: &str, _session_key: &str) -> Result<String> {
    // Convert to CID
    let cid_str = if content_hash.starts_with("Qm") {
        content_hash
    } else {
        &hash_to_cid(content_hash)?
    };

    let cid = Cid::try_from(cid_str)?;
    tracing::debug!("Generated CID: {}", cid);

    // Create reqwest client with timeout
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .build()?;

    // Use local IPFS gateway
    let gateway =
        env::var("IPFS_GATEWAY").expect("IPFS Gateway not set in the environment variables");
    let url = format!("{}{}", gateway, cid);

    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let data = response.text().await?;
        // Validate Base64
        let _ = BASE64
            .decode(&data)
            .map_err(|e| format!("Response is not valid Base64: {}", e));
        // Log the successful fetch
        tracing::debug!("Fetched from: {}", url);
        Ok(data)
    } else {
        return Err(eyre::eyre!(
            "Failed to fetch from {}: {}",
            url,
            response.status()
        ));
    }
}

// Fetch and decrypt IPFS data to UTF-8
#[tracing::instrument(name = "get_from_ipfs", skip(session_key))]
pub async fn get_from_ipfs(content_hash: &str, session_key: &str) -> Result<String> {
    // Fetch raw Base64-encoded data
    let base64_data = get_from_ipfs_raw(content_hash, session_key).await?;

    // Decode Base64
    let decoded_data = BASE64
        .decode(&base64_data)
        .map_err(|e| format!("Base64 decode error: {}", e))
        .unwrap_or_default();

    // Decrypt to UTF-8
    let decrypted_data = decrypt_utf8_data(&decoded_data, session_key);

    Ok(decrypted_data)
}
