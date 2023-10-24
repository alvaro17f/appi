use crate::{
    api::aur::AUR,
    modules::{aur_download::aur_download, github_search::github_search},
    utils::macros::error,
};
use anyhow::Result;
use color_print::{cformat, cprintln};
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use indicatif::ProgressBar;
use std::{cmp::Ordering, process::exit, time::Duration};

pub async fn aur_search(query: &str) -> Result<()> {
    let query = query.trim();

    let search_url = format!(
        "https://aur.archlinux.org/rpc/?v=5&type=search&arg={}",
        query
    );

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_message(cformat!("<y>Searching <c>{}<y>...", query));

    let response = AUR::get(&search_url).await?;

    #[allow(non_snake_case)]
    let _ = match response.results {
        Some(items) => match items.len() {
            0 => {
                pb.finish_and_clear();
                cprintln!("<r>No results found");
                if Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(cformat!("<y>do you want to try on github?"))
                    .default(true)
                    .interact()?
                {
                    github_search(query).await?;
                }
                Ok(())
            }
            _ => {
                let mut items = items;
                items.sort_by(|a, b| {
                    b.Popularity
                        .partial_cmp(&a.Popularity)
                        .unwrap_or(Ordering::Equal)
                });

                let selections: Vec<String> = items
                    .iter()
                    .filter_map(|x| {
                        if let (Some(Name), Some(Version)) =
                            (x.Name.as_deref(), x.Version.as_deref())
                        {
                            Some(cformat!("{}: <y>{}", Name, Version.split('-').next()?))
                        } else {
                            None
                        }
                    })
                    .take(5)
                    .collect();

                let mut selection_appimages: Vec<String> = Vec::new();
                for selection in selections.iter() {
                    if AUR::check_appimage(
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
                    cprintln!("<r>No results found");
                    if Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt(cformat!("<y>do you want to try on github?"))
                        .default(true)
                        .interact()?
                    {
                        github_search(query).await?
                    } else {
                        exit(0)
                    };
                }

                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt(cformat!("<y>select a package?"))
                    .default(0)
                    .max_length(10)
                    .items(&selection_appimages[..])
                    .interact()
                    .ok();

                if let Some(selection) = selection {
                    let name = &selection_appimages[selection]
                        .split(':')
                        .next()
                        .ok_or(error!("Failed to split selection"))
                        .unwrap()
                        .trim();

                    aur_download(name).await?
                };
                Ok(())
            }
        },
        None => Err(error!("No results")),
    };

    Ok(())
}
