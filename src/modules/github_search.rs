use crate::{modules::github_download::github_download, utils::macros::error};
use anyhow::Result;
use color_print::{cformat, cprintln};
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::ProgressBar;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    total_count: Option<u32>,
    items: Option<Vec<Items>>,
    assets: Option<Vec<Assets>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Items {
    full_name: Option<String>,
    description: Option<String>,
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
        headers.insert(
            "Accept",
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );
        let response = client.get(url).headers(headers).send().await?;
        let response = response.json::<Request>().await?;
        Ok(response)
    }
}

async fn check_appimage(full_name: &str) -> Result<bool> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", full_name);
    let response = Request::get(&url).await?;

    let assets = match response.assets {
        Some(assets) => assets,
        None => {
            return Ok(false);
        }
    };
    if assets
        .iter()
        .find(|a| a.name.as_ref().unwrap().ends_with(".AppImage"))
        .is_some()
    {
        return Ok(true);
    }

    Ok(false)
}

pub async fn github_search(query: &str) -> Result<()> {
    let query = query.trim();

    let search_url = format!("https://api.github.com/search/repositories?q={}", query);

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_message(cformat!("<c>Searching <c>{}...", query));

    let response = Request::get(&search_url).await?;

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
                    if check_appimage(
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
                    return Ok(());
                }

                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt(cformat!("<y>select a repository?"))
                    .default(0)
                    .max_length(10)
                    .items(&selection_appimages[..])
                    .interact()?;

                Ok(github_download(
                    selection_appimages[selection]
                        .split(':')
                        .next()
                        .ok_or(error!("Failed to split selection"))
                        .unwrap()
                        .trim(),
                )
                .await?)
            }
        },
        None => Err(error!("No results")),
    };

    Ok(result?)
}
