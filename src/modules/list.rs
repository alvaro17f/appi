use std::{fs, io};

use anyhow::{Ok, Result};
use color_print::cprintln;

use crate::utils::tools::get_user;

pub async fn list() -> Result<()> {
    cprintln!("<g,s>APPI</> - <y>AppImage Installer</>\n");
    let base_path = format!("/home/{}/Applications", get_user()?);
    let mut repo_entries = fs::read_dir(&base_path)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    repo_entries.sort();

    for repo_entry in repo_entries {
        let repo_path = repo_entry;
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
            let _creator = parts[1];
            let version = parts[2].trim_start_matches('v');
            cprintln!("<c,s>{}</> <y>{}", name, version);
        }
    }
    println!();
    Ok(())
}
