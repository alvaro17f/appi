use std::{fs, time::Duration};

use anyhow::{Ok, Result};
use color_print::{cformat, cprintln};
use indicatif::ProgressBar;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
    modules::{
        aur_download::aur_download, aur_search::get_appimage_url, github_download::github_download,
    },
    utils::{macros::error, tools::get_user},
};
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

pub async fn get_latest_version_aur(name: &str) -> Result<Version> {
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

async fn get_latest_version_github(name: &str, creator: &str) -> Result<Version> {
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

pub async fn update() -> Result<()> {
    let base_path = format!("/home/{}/Applications", get_user()?);
    let repo_entries = fs::read_dir(&base_path)?;
    for repo_entry in repo_entries {
        let repo_path = repo_entry?.path();
        if !repo_path.is_dir() {
            continue;
        }
        let dir_entries = fs::read_dir(&repo_path)?;
        for entry in dir_entries {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }
            let file_name = path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
                .to_lowercase()
                .replace(".appimage", "");

            let parts: Vec<&str> = file_name.split('-').collect();
            let name = parts[0];
            let creator = parts[1];
            let version = parts[2].trim_start_matches('v');
            let appimage = cformat!("<c,s>{}</> <y>{}", name, version);

            let pb = ProgressBar::new_spinner();
            pb.enable_steady_tick(Duration::from_millis(120));
            pb.set_message(cformat!("<y>{} <c>- Checking for updates...", name));

            let appimage_version = Version::parse(version)?;
            #[allow(clippy::needless_late_init)]
            let latest_version;
            if creator.to_lowercase() == "aur" {
                latest_version = get_latest_version_aur(&name.replace('_', "-")).await?;
            } else {
                latest_version =
                    get_latest_version_github(&name.replace('_', "-"), creator).await?;
            };

            if appimage_version < latest_version {
                pb.finish_and_clear();
                cprintln!("{} <r>is outdated</>", appimage);
                fs::remove_dir_all(format!("{base_path}/{name}"))?;

                if creator.to_lowercase() == "aur" {
                    let name = &name.replace('_', "-");
                    let appimage_url = get_appimage_url(name).await?;
                    aur_download(&appimage_url, name, latest_version.to_string().as_str()).await?;
                } else {
                    let name = &name.replace('_', "-");
                    let url = &format!("{}/{}", &creator, &name);
                    github_download(url).await?;
                }
            } else {
                pb.finish_and_clear();
                cprintln!("{} <g>is up to date</>", appimage);
            }
        }
    }
    Ok(())
}
