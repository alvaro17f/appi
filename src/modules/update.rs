#![allow(clippy::needless_late_init)]

use crate::{
    api::{aur::AUR, github::GITHUB},
    utils::tools::Tools,
};
use anyhow::{Ok, Result};
use color_print::{cformat, cprintln};
use indicatif::ProgressBar;
use semver::Version;
use std::{fs, time::Duration};

pub async fn update() -> Result<()> {
    let base_path = format!("/home/{}/Applications", Tools.get_user()?);
    let repo_entries = fs::read_dir(&base_path)?;
    for repo_entry in repo_entries {
        let repo_path = repo_entry?.path();
        if !repo_path.is_dir() {
            continue;
        }
        let dir_entries = fs::read_dir(&repo_path)?;
        for entry in dir_entries {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }
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
            let version = parts[2].trim_start_matches('v');
            let appimage = cformat!("<c,s>{}</> <y>{}", name, version);

            let pb = ProgressBar::new_spinner();
            pb.enable_steady_tick(Duration::from_millis(120));
            pb.set_message(cformat!("<y>{} <c>- Checking for updates...", name));

            let appimage_version = Version::parse(version)?;
            let latest_version;
            if creator.to_lowercase() == "aur" {
                latest_version = AUR::get_latest_version(&name.replace('_', "-")).await?;
            } else {
                latest_version =
                    GITHUB::get_latest_version(&name.replace('_', "-"), &creator.replace('_', "-"))
                        .await?;
            };

            if appimage_version < latest_version {
                pb.finish_and_clear();
                cprintln!("{} <r>is outdated</>", appimage);
                fs::remove_dir_all(format!("{base_path}/{name}"))?;

                if creator.to_lowercase() == "aur" {
                    let name = &name.replace('_', "-");
                    AUR::download(name).await?;
                } else {
                    let name = &name.replace('_', "-");
                    let creator = &creator.replace('_', "-");
                    let url = &format!("{}/{}", &creator, &name);
                    GITHUB::download(url).await?;
                }
            } else {
                pb.finish_and_clear();
                cprintln!("{} <g>is up to date</>", appimage);
            }
        }
    }
    Ok(())
}
