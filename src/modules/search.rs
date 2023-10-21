use crate::{modules::github::github, utils::macros::error};
use anyhow::{Ok, Result};
use color_print::cformat;
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::ProgressBar;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    total_count: Option<u32>,
    items: Option<Vec<Items>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Items {
    full_name: Option<String>,
    description: Option<String>,
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

async fn get_repo_url(search_url: &str) -> Result<Request> {
    let response = Request::get(search_url).await?;
    Ok(response)
}

pub async fn search(query: &str) -> Result<()> {
    let query = query.trim();

    let search_url = format!("https://api.github.com/search/repositories?q={}", query);

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_message(cformat!("<c>Searching <c>{}...", query));
    let response = get_repo_url(&search_url).await?;

    pb.finish_and_clear();

    let result = match response.items {
        Some(items) => match items.len() {
            0 => Err(error!("No results")),
            1 => Ok(github(items[0].full_name.as_deref().unwrap()).await?),
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
                    .collect();

                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt(cformat!("<y>select a repository?"))
                    .default(0)
                    .max_length(10)
                    .items(&selections[..])
                    .interact()?;

                Ok(github(
                    selections[selection]
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
