use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use scraper::{Html, Selector};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::utils::macros::error;

#[derive(Serialize, Deserialize, Debug)]
pub struct AUR {
    pub results: Option<Vec<Results>>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Results {
    pub Name: Option<String>,
    pub Popularity: Option<f32>,
    pub Version: Option<String>,
}

impl AUR {
    pub async fn get(url: &str) -> Result<Self> {
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));
        let response = client.get(url).headers(headers).send().await?;
        let response = response.json::<AUR>().await?;
        Ok(response)
    }

    pub async fn check_appimage(package_name: &str) -> Result<bool> {
        let body = reqwest::get(format!(
            "https://aur.archlinux.org/packages/{}/",
            package_name
        ))
        .await?
        .text()
        .await?;

        let document = Html::parse_document(&body);

        let selector = Selector::parse("a").unwrap();

        for element in document.select(&selector) {
            let a_text = element.text().collect::<String>();
            if a_text.to_lowercase().ends_with(".appimage") {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub async fn get_appimage_url(package_name: &str) -> Result<String> {
        let body = reqwest::get(format!(
            "https://aur.archlinux.org/packages/{}/",
            package_name
        ))
        .await?
        .text()
        .await?;

        let document = Html::parse_document(&body);

        let selector = Selector::parse("a").unwrap();

        let mut appimage_url: String = String::new();
        for element in document.select(&selector) {
            let a_link = element.value().attr("href").unwrap();
            let a_text = element.text().collect::<String>();
            if !a_text.to_lowercase().contains("arm64")
                && a_text.to_lowercase().ends_with(".appimage")
            {
                appimage_url = a_link.to_string();
            }
        }
        Ok(appimage_url)
    }

    pub async fn get_latest_version(name: &str) -> Result<Version> {
        let url = format!("https://aur.archlinux.org/rpc/v5/info/{}", name);
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "reqwest".parse().unwrap());

        let response = AUR::get(&url).await?;
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
}
