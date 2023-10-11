use std::fs;

use anyhow::{Ok, Result};
use color_print::{cformat, cprintln};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::utils::tools::get_user;

#[derive(Serialize, Deserialize, Debug)]
struct Release {
    tag_name: Option<String>,
}

impl Release {
    async fn get(url: &str) -> Result<Self> {
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));
        let response = client.get(url).headers(headers).send().await?;
        let response = response.json::<Release>().await?;
        Ok(response)
    }
}
async fn get_latest_version(name: &str, creator: &str) -> Result<Version> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        creator, name
    );
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "reqwest".parse().unwrap());

    let release = Release::get(&url).await?;
    let tag_name = release.tag_name.unwrap_or_default();
    let version_string = tag_name.trim_start_matches('v');
    let latest_version = Version::parse(&version_string)?;
    Ok(latest_version)
}
pub async fn update() -> Result<()> {
    let dir_path = format!("/home/{}/Applications", get_user()?);
    let dir_entries = fs::read_dir(dir_path.to_owned())?;
    for entry in dir_entries {
        let path = entry?.path();
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
        let version = parts[2];
        let appimage = cformat!("<c,s>{}</> <y>{}", name, version);
        let latest_version = get_latest_version(name, creator).await?;
        let appimage_version = Version::parse(version)?;
        if appimage_version < latest_version {
            cprintln!("{} <r>is outdated</>", appimage);
        }
    }
    Ok(())
}
