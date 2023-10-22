use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{Ok, Result};
use color_print::{cformat, cprintln};
use dialoguer::{theme::ColorfulTheme, Select};

use crate::utils::tools::get_user;

pub async fn delete() -> Result<()> {
    cprintln!("<g,s>APPI</> - <y>AppImage Installer</>\n");
    let dir_path = format!("/home/{}/Applications", get_user()?);
    let dir_entries = fs::read_dir(&dir_path)?;
    let mut installed = HashMap::new();
    for entry in dir_entries {
        let path = entry?.path();
        let file_name = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
            .to_lowercase();

        installed.insert(file_name.clone(), path.clone());
    }

    if installed.is_empty() {
        cprintln!("<r>no appimages installed</>");
        println!();
        return Ok(());
    }

    let mut selections: Vec<String> = installed.keys().cloned().collect();
    selections.sort();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(cformat!("<y>select an appimage to remove?"))
        .default(0)
        .max_length(10)
        .items(&selections[..])
        .interact()?;
    let selected_app = &selections[selection];

    let app_folder = installed.get(selected_app).unwrap();

    fs::remove_dir_all(app_folder)?;

    let app_path = PathBuf::from(format!("/home/{}/.local/share/applications", get_user()?));

    let matching_file = fs::read_dir(app_path)?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file()
                && path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .contains(selected_app)
            {
                Some(path)
            } else {
                None
            }
        })
        .next();

    if let Some(path) = matching_file {
        fs::remove_file(path)?;
    } else {
        eprintln!("No matching file found for {}", selected_app);
    }
    Ok(())
}
