use std::fs;

use anyhow::{Ok, Result};
use color_print::cprintln;

use crate::utils::tools::get_user;

pub async fn list() -> Result<()> {
    cprintln!("<g,s>APPI</> - <y>AppImage Installer</>\n");
    let dir_path = format!("/home/{}/Applications", get_user()?);
    let dir_entries = fs::read_dir(dir_path)?;
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
        cprintln!("<c,s>{}</> <y>{}", name, version);
    }
    println!();
    Ok(())
}
