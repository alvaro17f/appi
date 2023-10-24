use anyhow::Result;
use chrono::{TimeZone, Utc};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::utils::macros::error;

#[derive(Serialize, Deserialize, Debug)]
pub struct GITHUB {
    pub total_count: Option<u32>,
    pub items: Option<Vec<Items>>,
    pub url: Option<String>,
    pub tag_name: Option<String>,
    pub assets: Option<Vec<Assets>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Items {
    pub full_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Assets {
    pub name: Option<String>,
    pub browser_download_url: Option<String>,
}
#[derive(Deserialize)]
struct RateLimit {
    rate: Rate,
}

#[derive(Deserialize)]
struct Rate {
    remaining: u32,
    reset: i64,
}

impl GITHUB {
    pub async fn get(url: &str) -> Result<Self> {
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
        let response = client.get(url).headers(headers).send().await?;
        let response = response.json::<GITHUB>().await?;
        Ok(response)
    }

    pub async fn get_latest_version(name: &str, creator: &str) -> Result<Version> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            creator, name
        );
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "reqwest".parse().unwrap());

        let release = GITHUB::get(&url).await?;
        let tag_name = release.tag_name.unwrap_or_default();
        let version_string = tag_name.trim_start_matches('v');
        let latest_version = Version::parse(version_string)?;
        Ok(latest_version)
    }

    pub async fn check_rate_limit() -> Result<()> {
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
}
