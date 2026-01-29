use anyhow::{bail, Context, Result};
use std::process::Command;
use crate::model::settings::Settings;

pub struct DependencyStatus {
    pub yt_dlp_version: String,
    pub mpv_installed: bool,
}

pub fn check_dependencies(settings: &Settings) -> Result<DependencyStatus> {
    let version = check_yt_dlp(settings)?;
    let mpv = check_mpv(settings)?;

    Ok(DependencyStatus {
        yt_dlp_version: version,
        mpv_installed: mpv,
    })
}

fn check_yt_dlp(settings: &Settings) -> Result<String> {
    let output = Command::new(&settings.ytdlp_path)
        .arg("--version")
        .output()
        .context("Failed to execute yt-dlp. Is it installed and in your PATH?")?;

    if !output.status.success() {
        bail!("yt-dlp command failed with status: {}", output.status);
    }

    let version_str = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(version_str)
}

fn check_mpv(settings: &Settings) -> Result<bool> {
    // Just check availability
    let output = Command::new(&settings.mpv_path).arg("--version").output();

    match output {
        Ok(o) => Ok(o.status.success()),
        Err(_) => Ok(false),
    }
}
