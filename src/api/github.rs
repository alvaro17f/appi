use crate::utils::{appimage::AppImage, errors::error, tools::Tools};
use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};
use color_print::{cformat, cprintln};
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::ProgressBar;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{process::exit, time::Duration};

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

    async fn get_response(url: &str) -> Result<(String, String)> {
        let response = GITHUB::get(url).await?;

        let version = response.tag_name.context(error!("No version found"))?;
        let version = version
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect::<String>();

        let assets = response.assets.unwrap();
        let appimage_assets: Vec<_> = assets
            .iter()
            .filter(|a| {
                a.name
                    .as_ref()
                    .unwrap()
                    .to_lowercase()
                    .ends_with(".appimage")
            })
            .collect();

        if appimage_assets.is_empty() {
            return Err(error!("No AppImage found"));
        } else if appimage_assets.len() == 1 {
            let asset = appimage_assets[0];
            let appimage_url = asset
                .browser_download_url
                .as_ref()
                .context(error!("No URL to AppImage found"))?
                .to_string();
            return Ok((appimage_url, version));
        }

        let items: Vec<&str> = appimage_assets
            .iter()
            .map(|a| a.name.as_ref().unwrap().as_str())
            .collect();
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(cformat!("<y>multiple appimages found, please select one:"))
            .default(0)
            .items(&items)
            .interact()
            .ok();

        match selection {
            Some(index) => {
                let asset = appimage_assets[index];
                let appimage_url = asset
                    .browser_download_url
                    .as_ref()
                    .context(error!("No URL to AppImage found"))?
                    .to_string();
                Ok((appimage_url, version))
            }
            None => exit(0),
        }
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

    async fn check_appimage(full_name: &str) -> Result<bool> {
        let url = format!("https://api.github.com/repos/{}/releases/latest", full_name);
        let response = GITHUB::get(&url).await?;

        let assets = match response.assets {
            Some(assets) => assets,
            None => {
                return Ok(false);
            }
        };
        if assets.iter().any(|a| {
            a.name
                .as_ref()
                .unwrap()
                .to_lowercase()
                .ends_with(".appimage")
        }) {
            return Ok(true);
        }

        Ok(false)
    }

    pub async fn download(repo_url: &str) -> Result<()> {
        GITHUB::check_rate_limit().await?;
        let repo_url = repo_url.trim_end_matches('/');
        let repo_parts: Vec<&str> = repo_url.split('/').collect();
        let owner = repo_parts[repo_parts.len() - 2].to_string();
        let repo = repo_parts[repo_parts.len() - 1].to_string();
        let owner_name = owner.replace('-', "_");
        let repo_name = repo.replace('-', "_");

        let app_folder = format!("/home/{}/Applications/{}", Tools.get_user()?, repo_name);
        let app_folder_path = std::path::Path::new(&app_folder);
        if app_folder_path.exists() {
            cprintln!("<c>{} <y>is already installed", repo_name);
            return Ok(());
        }

        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        );

        let (appimage_url, version) = GITHUB::get_response(&url).await?;

        let file_path = format!(
            "/home/{}/Applications/{}/{}-{}-v{}.appimage",
            Tools.get_user()?,
            repo_name,
            repo_name,
            owner_name,
            version
        );

        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_message(cformat!("<c>Downloading {}...", repo_name));
        AppImage.download(&appimage_url, &file_path).await?;
        pb.finish_and_clear();

        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_message(cformat!("<c>Installing {}...", repo_name));
        AppImage.extract(&file_path)?;
        AppImage.integrate(&file_path, &repo_name)?;
        pb.finish_and_clear();

        cprintln!(
            "<g>Successfully installed <c>{}</c> <g>version <c>{}</c></g>",
            repo_name,
            version
        );
        Ok(())
    }

    pub async fn search(query: &str) -> Result<()> {
        GITHUB::check_rate_limit().await?;
        let query = query.trim();

        let search_url = format!("https://api.github.com/search/repositories?q={}", query);

        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_message(cformat!(
            "<y>Searching</> <c,s>{}</> <y>on</> <m,s>github</><y>...</>",
            query
        ));

        let response = GITHUB::get(&search_url).await?;

        let result = match response.items {
            Some(items) => match items.len() {
                0 => Err(error!("No results")),
                _ => {
                    let selections: Vec<String> = items
                        .iter()
                        .filter_map(|x| {
                            if let (Some(full_name), Some(description)) =
                                (x.full_name.as_deref(), x.description.as_deref())
                            {
                                Some(cformat!("{}: <y>{}", full_name, description))
                            } else {
                                None
                            }
                        })
                        .take(5)
                        .collect();

                    let mut selection_appimages: Vec<String> = Vec::new();
                    for selection in selections.iter() {
                        if GITHUB::check_appimage(
                            selection
                                .split(':')
                                .next()
                                .ok_or(error!("Failed to split selection"))
                                .unwrap()
                                .trim(),
                        )
                        .await?
                        {
                            selection_appimages.push(selection.to_string());
                        } else {
                            continue;
                        }
                    }

                    pb.finish_and_clear();

                    if selection_appimages.is_empty() {
                        cprintln!("<r>No repositories found with AppImages</>");
                    }

                    let selection = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt(cformat!("<y>select a repository?"))
                        .default(0)
                        .max_length(10)
                        .items(&selection_appimages[..])
                        .interact()
                        .ok();

                    if let Some(selection) = selection {
                        GITHUB::download(
                            selection_appimages[selection]
                                .split(':')
                                .next()
                                .ok_or(error!("Failed to split selection"))
                                .unwrap()
                                .trim(),
                        )
                        .await?
                    }

                    Ok(())
                }
            },
            None => Err(error!("No results")),
        };

        result
    }
}
