use crate::utils::macros::error;
use anyhow::Result;
use chrono::{TimeZone, Utc};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;

#[derive(Deserialize)]
struct RateLimit {
    rate: Rate,
}

#[derive(Deserialize)]
struct Rate {
    remaining: u32,
    reset: i64,
}

pub async fn check_github_rate_limit() -> Result<()> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));
    headers.insert(
        "Accept",
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static("2022-11-28"),
    );
    let response = client
        .get("https://api.github.com/rate_limit")
        .headers(headers)
        .send()
        .await?
        .json::<RateLimit>()
        .await?;

    if response.rate.remaining == 0 {
        let reset_time = Utc.timestamp_opt(response.rate.reset as i64, 0).unwrap();
        let remaining_time = reset_time.signed_duration_since(Utc::now());
        let remaining_minutes = remaining_time.num_minutes();
        return Err(error!(format!(
            "Github rate limit exceeded. Wait for {} min and try again",
            remaining_minutes
        )));
    }

    Ok(())
}
