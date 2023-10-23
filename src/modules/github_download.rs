use crate::utils::{
    appimage_tools::{download_appimage, extract_appimage, integrate_appimage},
    macros::error,
    rate_limit::check_github_rate_limit,
    tools::get_user,
};
use anyhow::{Context, Ok, Result};
use color_print::{cformat, cprintln};
use indicatif::ProgressBar;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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

async fn get_response(url: &str) -> Result<(String, String)> {
    let response = Request::get(url).await?;

    let version = response.tag_name.context(error!("No version found"))?;
    let version = version
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>();

    let assets = response.assets.unwrap();
    let asset = assets
        .iter()
        .find(|a| a.name.as_ref().unwrap().ends_with(".AppImage"))
        .context(error!("No AppImage found"))?;
    let appimage_url = asset
        .browser_download_url
        .as_ref()
        .context(error!("No URL to AppImage found"))?
        .to_string();

    Ok((appimage_url, version))
}

pub async fn github_download(repo_url: &str) -> Result<()> {
    check_github_rate_limit().await?;
    let repo_url = repo_url.trim_end_matches('/');
    let repo_parts: Vec<&str> = repo_url.split('/').collect();
    let owner = repo_parts[repo_parts.len() - 2].to_string();
    let repo = repo_parts[repo_parts.len() - 1].to_string();
    let owner_name = owner.replace('-', "_");
    let repo_name = repo.replace('-', "_");

    let app_folder = format!("/home/{}/Applications/{}", get_user()?, repo_name);
    let app_folder_path = std::path::Path::new(&app_folder);
    if app_folder_path.exists() {
        cprintln!("<c>{} <y>is already installed", repo_name);
        return Ok(());
    }

    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        owner, repo
    );

    let (appimage_url, version) = get_response(&url).await?;

    let file_path = format!(
        "/home/{}/Applications/{}/{}-{}-v{}.appimage",
        get_user()?,
        repo_name,
        repo_name,
        owner_name,
        version
    );

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_message(cformat!("<c>Downloading {}...", repo_name));
    download_appimage(&appimage_url, &file_path).await?;
    pb.finish_and_clear();

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_message(cformat!("<c>Installing {}...", repo_name));
    extract_appimage(&file_path)?;
    integrate_appimage(&file_path, &repo_name)?;
    pb.finish_and_clear();

    cprintln!(
        "<g>Successfully installed <c>{}</c> <g>version <c>{}</c></g>",
        repo_name,
        version
    );
    Ok(())
}
