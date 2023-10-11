use std::{collections::HashMap, fs};

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
            .to_lowercase()
            .replace(".appimage", "");
        let parts: Vec<&str> = file_name.split('-').collect();
        let name = parts[0];
        let _creator = parts[1];
        let version = parts[2];
        let appimage = cformat!("<c,s>{}</> <y>{}", name, version);
        installed.insert(appimage.clone(), path.clone());
    }

    if installed.is_empty() {
        cprintln!("<r>no appimages installed</>");
        println!();
        return Ok(());
    }

    let selections: Vec<String> = installed.keys().cloned().collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(cformat!("<y>select an appimage to remove?"))
        .default(0)
        .max_length(10)
        .items(&selections[..])
        .interact()?;
    let selected_appimage = &selections[selection];
    let file_path = installed.get(selected_appimage).unwrap();

    fs::remove_file(file_path)?;
    Ok(())
}
