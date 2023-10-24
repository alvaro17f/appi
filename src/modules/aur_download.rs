use crate::{
    api::aur::AUR,
    utils::{appimage::AppImage, tools::get_user},
};
use anyhow::{Ok, Result};
use color_print::{cformat, cprintln};
use indicatif::ProgressBar;
use std::time::Duration;

pub async fn aur_download(name: &str) -> Result<()> {
    let appimage_url = AUR::get_appimage_url(name).await?;
    let version = AUR::get_latest_version(name).await?;
    let name = name.replace('-', "_");

    let app_folder = format!("/home/{}/Applications/{}", get_user()?, name);
    let app_folder_path = std::path::Path::new(&app_folder);
    if app_folder_path.exists() {
        cprintln!("<c>{} <y>is already installed", name);
        return Ok(());
    }

    let file_path = format!(
        "/home/{}/Applications/{}/{}-AUR-v{}.appimage",
        get_user()?,
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
