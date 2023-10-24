use crate::{
    api::github::GITHUB,
    utils::{appimage::AppImage, errors::error, tools::Tools},
};
use anyhow::Result;
use color_print::{cformat, cprintln};
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use indicatif::ProgressBar;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use scraper::{Html, Selector};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, process::exit, time::Duration};

#[derive(Serialize, Deserialize, Debug)]
pub struct AUR {
    pub results: Option<Vec<Results>>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Results {
    pub Name: Option<String>,
    pub Popularity: Option<f32>,
    pub Version: Option<String>,
}

impl AUR {
    pub async fn get(url: &str) -> Result<Self> {
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));
        let response = client.get(url).headers(headers).send().await?;
        let response = response.json::<AUR>().await?;
        Ok(response)
    }

    pub async fn check_appimage(package_name: &str) -> Result<bool> {
        let body = reqwest::get(format!(
            "https://aur.archlinux.org/packages/{}/",
            package_name
        ))
        .await?
        .text()
        .await?;

        let document = Html::parse_document(&body);

        let selector = Selector::parse("a").unwrap();

        for element in document.select(&selector) {
            let a_text = element.text().collect::<String>();
            if a_text.to_lowercase().ends_with(".appimage") {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub async fn get_appimage_url(package_name: &str) -> Result<String> {
        let body = reqwest::get(format!(
            "https://aur.archlinux.org/packages/{}/",
            package_name
        ))
        .await?
        .text()
        .await?;

        let document = Html::parse_document(&body);

        let selector = Selector::parse("a").unwrap();

        let mut appimage_url: String = String::new();
        for element in document.select(&selector) {
            let a_link = element.value().attr("href").unwrap();
            let a_text = element.text().collect::<String>();
            if a_text.to_lowercase().ends_with(".appimage") {
                appimage_url = a_link.to_string();
            }
        }
        Ok(appimage_url)
    }

    pub async fn get_latest_version(name: &str) -> Result<Version> {
        let url = format!("https://aur.archlinux.org/rpc/v5/info/{}", name);
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "reqwest".parse().unwrap());

        let response = AUR::get(&url).await?;
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

    pub async fn download(name: &str) -> Result<()> {
        let appimage_url = AUR::get_appimage_url(name).await?;
        let version = AUR::get_latest_version(name).await?;
        let name = name.replace('-', "_");

        let app_folder = format!("/home/{}/Applications/{}", Tools.get_user()?, name);
        let app_folder_path = std::path::Path::new(&app_folder);
        if app_folder_path.exists() {
            cprintln!("<c>{} <y>is already installed", name);
            return Ok(());
        }

        let file_path = format!(
            "/home/{}/Applications/{}/{}-AUR-v{}.appimage",
            Tools.get_user()?,
            name,
            name,
            version
        );

        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_message(cformat!("<c>Downloading {}...", name));
        AppImage.download(&appimage_url, &file_path).await?;
        pb.finish_and_clear();

        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_message(cformat!("<c>Installing {}...", name));
        AppImage.extract(&file_path)?;
        AppImage.integrate(&file_path, &name)?;
        pb.finish_and_clear();

        cprintln!(
            "<g>Successfully installed <c>{}</c> <g>version <c>{}</c></g>",
            name,
            version
        );
        Ok(())
    }

    pub async fn search(query: &str) -> Result<()> {
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
                        GITHUB::search(query).await?;
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
                            GITHUB::search(query).await?
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

                        AUR::download(name).await?
                    };
                    Ok(())
                }
            },
            None => Err(error!("No results")),
        };

        Ok(())
    }
}
