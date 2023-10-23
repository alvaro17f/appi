use crate::utils::{macros::error, tools::get_user};
use anyhow::Result;
use std::{
    fs::{self, Permissions},
    os::unix::prelude::PermissionsExt,
    path::PathBuf,
    process::Command,
};
use tokio::{
    fs::{create_dir_all, set_permissions, File},
    io::AsyncWriteExt,
};

pub async fn download_appimage(url: &str, file_path: &str) -> Result<()> {
    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        return Err(error!(
            "Failed to download file. Check if the package is available"
        ));
    }

    let dir_path = std::path::Path::new(&file_path).parent().unwrap();
    create_dir_all(dir_path).await?;
    let mut output = File::create(&file_path).await?;
    let bytes = response.bytes().await?;
    output.write_all(&bytes).await?;

    let permissions = Permissions::from_mode(0o755);
    set_permissions(file_path, permissions).await?;
    Ok(())
}

pub fn extract_appimage(file_path: &str) -> Result<()> {
    let dir_path = std::path::Path::new(&file_path).parent().unwrap();
    std::env::set_current_dir(dir_path)?;
    Command::new(file_path).arg("--appimage-extract").output()?;

    Ok(())
}

pub fn integrate_appimage(file_path: &str, name: &str) -> Result<()> {
    let desktop_applications_path = format!("/home/{}/.local/share/applications", get_user()?);
    let desktop_applications_path = std::path::Path::new(&desktop_applications_path);

    let appimage_path = std::path::PathBuf::from(file_path);
    let appimage_dir = appimage_path.parent().unwrap();
    let appimage_extracted_dir = appimage_dir.join("squashfs-root");
    let exec_path = appimage_extracted_dir.join("AppRun");

    let entries = appimage_extracted_dir.clone();

    let desktop_file = entries
        .read_dir()?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() && path.extension().unwrap_or_default() == "desktop" {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .next()
        .ok_or_else(|| error!("No desktop file found"))?;

    let icon_extensions = ["svg", "png", "jpg", "jpeg", "bmp", "ico", "webp"];
    let mut icon = "None".to_string();
    for entry in entries.read_dir()? {
        let path = entry.as_ref().unwrap().path();
        if path.is_file() {
            let extension = path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if icon_extensions.iter().any(|&ext| ext == extension) {
                icon = path.to_string_lossy().to_string();
                break;
            }
        }
    }

    if icon == "None" {
        return Err(error!("No icon found"));
    }

    let desktop_file_name = format!("{}.desktop", name.to_lowercase());
    let desktop_app_path = PathBuf::from(desktop_applications_path).join(desktop_file_name);

    fs::copy(desktop_file, desktop_app_path)?;

    let desktop_file_name = format!("{}.desktop", name.to_lowercase());
    let desktop_app_path = PathBuf::from(desktop_applications_path).join(desktop_file_name);

    let mut desktop_file_content = std::fs::read_to_string(&desktop_app_path)?;
    let lines: Vec<_> = desktop_file_content
        .lines()
        .map(|line| {
            if line.starts_with("Icon=") {
                format!("Icon={}", icon)
            } else if line.starts_with("Exec=") {
                format!("Exec={} %U", exec_path.display())
            } else {
                line.to_string()
            }
        })
        .collect();

    desktop_file_content = lines.join("\n");

    std::fs::write(&desktop_app_path, desktop_file_content)?;

    Ok(())
}
