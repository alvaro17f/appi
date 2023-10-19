use crate::utils::{macros::error, tools::get_user};
use anyhow::{Ok, Result};
use color_print::{cformat, cprintln};
use indicatif::ProgressBar;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::{
    borrow::BorrowMut,
    fs::{self, Permissions},
    os::unix::prelude::PermissionsExt,
    path::PathBuf,
    process::Command,
    time::Duration,
};
use tokio::{
    fs::{create_dir_all, set_permissions, File},
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

async fn get_response(url: &str) -> Result<(String, String)> {
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

async fn download_appimage(url: &str, file_path: &str) -> Result<()> {
    let response = reqwest::get(url).await?;

    let dir_path = std::path::Path::new(&file_path).parent().unwrap();
    create_dir_all(dir_path).await?;
    let mut output = File::create(&file_path).await?;
    let bytes = response.bytes().await?;
    output.write_all(&bytes).await?;

    let permissions = Permissions::from_mode(0o755);
    set_permissions(file_path, permissions).await?;
    Ok(())
}

fn extract_appimage(file_path: &str) -> Result<()> {
    let dir_path = std::path::Path::new(&file_path).parent().unwrap();
    std::env::set_current_dir(dir_path)?;
    Command::new(file_path).arg("--appimage-extract").output()?;

    Ok(())
}

fn integrate_appimage(file_path: &str, repo_name: &str) -> Result<()> {
    let desktop_applications_path = format!("/home/{}/.local/share/applications", get_user()?);
    let desktop_applications_path = std::path::Path::new(&desktop_applications_path);

    let appimage_path = std::path::PathBuf::from(file_path);
    let appimage_dir = appimage_path.parent().unwrap();
    let appimage_extracted_dir = appimage_dir.join("squashfs-root");
    let exec_path = appimage_extracted_dir.join("AppRun");

    let mut entries = fs::read_dir(appimage_extracted_dir)?;

    let desktop_file = entries
        .borrow_mut()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() && path.extension().unwrap_or_default() == "desktop" {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .next()
        .ok_or_else(|| error!("No desktop file found"))?;

    let icon = entries
        .borrow_mut()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() && image::ImageFormat::from_path(&path).is_ok() {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .next()
        .ok_or_else(|| error!("No icon found"))?;

    let desktop_file_name = format!("{}.desktop", repo_name);
    let desktop_app_path = PathBuf::from(desktop_applications_path).join(desktop_file_name);

    fs::copy(desktop_file, desktop_app_path)?;

    let desktop_file_name = format!("{}.desktop", repo_name);
    let desktop_app_path = PathBuf::from(desktop_applications_path).join(desktop_file_name);

    let mut desktop_file_content = std::fs::read_to_string(&desktop_app_path)?;
    let lines: Vec<_> = desktop_file_content
        .lines()
        .map(|line| {
            if line.starts_with("Icon=") {
                format!("Icon={}", icon)
            } else if line.starts_with("Exec=") {
                format!("Exec={} %U", exec_path.display())
            } else {
                line.to_string()
            }
        })
        .collect();

    desktop_file_content = lines.join("\n");

    std::fs::write(&desktop_app_path, desktop_file_content)?;

    Ok(())
}

pub async fn github(repo_url: &str) -> Result<()> {
    let repo_parts: Vec<&str> = repo_url.split('/').collect();
    let owner = repo_parts[repo_parts.len() - 2].to_string();
    let repo = repo_parts[repo_parts.len() - 1].to_string();
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
        owner,
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
