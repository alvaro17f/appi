use crate::utils::tools::get_user;
use anyhow::{Ok, Result};
use color_print::cprintln;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::{fs::Permissions, os::unix::prelude::PermissionsExt};
use tokio::{
    fs::{set_permissions, File},
    io::AsyncWriteExt,
};

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    url: Option<String>,
    tag_name: Option<String>,
    assets: Option<Vec<Assets>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Assets {
    name: Option<String>,
    browser_download_url: Option<String>,
}

impl Request {
    async fn get(url: &str) -> Result<Self> {
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));
        let response = client.get(url).headers(headers).send().await?;
        let response = response.json::<Request>().await?;
        Ok(response)
    }
}

pub async fn get_response(url: &str) -> Result<(String, String)> {
    let response = Request::get(url).await?;

    let version = response.tag_name.unwrap();
    let version = version
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>();

    let assets = response.assets.unwrap();
    let asset = assets
        .iter()
        .find(|a| a.name.as_ref().unwrap().ends_with(".AppImage"))
        .unwrap();
    let appimage_url = asset.browser_download_url.as_ref().unwrap().to_string();

    Ok((appimage_url, version))
}

pub async fn download_appimage(url: &str, repo: &str, owner: &str, version: &str) -> Result<()> {
    let response = reqwest::get(url).await?;
    let file_path = format!(
        "/home/{}/Applications/{}-{}-v{}.appimage",
        get_user()?,
        repo,
        owner,
        version
    );
    let mut output = File::create(&file_path).await?;
    let bytes = response.bytes().await?;
    output.write_all(&bytes).await?;

    let permissions = Permissions::from_mode(0o755);
    set_permissions(file_path, permissions).await?;
    Ok(())
}

pub async fn github(repo_url: &str) -> Result<()> {
    let repo_parts: Vec<&str> = repo_url.split('/').collect();
    let owner = repo_parts[repo_parts.len() - 2].to_string();
    let repo = repo_parts[repo_parts.len() - 1].to_string();

    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        owner, repo
    );

    let (appimage_url, version) = get_response(&url).await?;
    download_appimage(&appimage_url, &repo, &owner, &version).await?;

    cprintln!(
        "<g>Successfully installed <c>{}</c> <g>version <c>{}</c></g>",
        repo,
        version
    );
    Ok(())
}
