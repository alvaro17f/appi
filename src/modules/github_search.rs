use crate::{api::github::GITHUB, modules::github_download::github_download, utils::macros::error};
use anyhow::Result;
use color_print::{cformat, cprintln};
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::ProgressBar;
use std::time::Duration;

async fn check_appimage(full_name: &str) -> Result<bool> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", full_name);
    let response = GITHUB::get(&url).await?;

    let assets = match response.assets {
        Some(assets) => assets,
        None => {
            return Ok(false);
        }
    };
    if assets
        .iter()
        .any(|a| a.name.as_ref().unwrap().ends_with(".AppImage"))
    {
        return Ok(true);
    }

    Ok(false)
}

pub async fn github_search(query: &str) -> Result<()> {
    GITHUB::check_rate_limit().await?;
    let query = query.trim();

    let search_url = format!("https://api.github.com/search/repositories?q={}", query);

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_message(cformat!("<y>Searching <c>{} <y>on <r>github<y>...", query));

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
                }

                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt(cformat!("<y>select a repository?"))
                    .default(0)
                    .max_length(10)
                    .items(&selection_appimages[..])
                    .interact()
                    .ok();

                if let Some(selection) = selection {
                    github_download(
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
