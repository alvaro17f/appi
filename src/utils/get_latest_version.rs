use anyhow::{Ok, Result};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::utils::macros::error;
#[derive(Serialize, Deserialize, Debug)]
struct AurRequest {
    results: Option<Vec<AurRelease>>,
}

#[allow(non_snake_case)] // Disable the warning for this struct
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct AurRelease {
    Name: Option<String>,
    Version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GithubRelease {
    tag_name: Option<String>,
}

impl AurRequest {
    async fn get_aur(url: &str) -> Result<Self> {
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));
        let response = client.get(url).headers(headers).send().await?;
        let response = response.json::<AurRequest>().await?;
        Ok(response)
    }
}

impl GithubRelease {
    async fn get_github(url: &str) -> Result<Self> {
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));
        let response = client.get(url).headers(headers).send().await?;
        let response = response.json::<GithubRelease>().await?;
        Ok(response)
    }
}

pub async fn get_latest_aur(name: &str) -> Result<Version> {
    let url = format!("https://aur.archlinux.org/rpc/v5/info/{}", name);
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "reqwest".parse().unwrap());

    let response = AurRequest::get_aur(&url).await?;
    let version = response
        .results
        .as_ref()
        .and_then(|results| results.get(0))
        .and_then(|result| result.Version.as_ref())
        .ok_or_else(|| error!("Failed to get version"))?
        .split('-')
        .next()
        .ok_or_else(|| error!("Failed to split version"))?
        .to_string();

    let latest_version = Version::parse(&version)?;
    Ok(latest_version)
}

pub async fn get_latest_github(name: &str, creator: &str) -> Result<Version> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        creator, name
    );
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "reqwest".parse().unwrap());

    let release = GithubRelease::get_github(&url).await?;
    let tag_name = release.tag_name.unwrap_or_default();
    let version_string = tag_name.trim_start_matches('v');
    let latest_version = Version::parse(version_string)?;
    Ok(latest_version)
}
