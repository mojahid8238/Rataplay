use anyhow::{bail, Context, Result};
use std::process::Command;

pub struct DependencyStatus {
    pub yt_dlp_version: String,
    pub mpv_installed: bool,
}

pub fn check_dependencies() -> Result<DependencyStatus> {
    let version = check_yt_dlp()?;
    let mpv = check_mpv()?;

    Ok(DependencyStatus {
        yt_dlp_version: version,
        mpv_installed: mpv,
    })
}

fn check_yt_dlp() -> Result<String> {
    let output = Command::new("yt-dlp")
        .arg("--version")
        .output()
        .context("Failed to execute yt-dlp. Is it installed and in your PATH?")?;

    if !output.status.success() {
        bail!("yt-dlp command failed with status: {}", output.status);
    }

    let version_str = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(version_str)
}

fn check_mpv() -> Result<bool> {
    // Just check availability
    let output = Command::new("mpv").arg("--version").output();

    match output {
        Ok(o) => Ok(o.status.success()),
        Err(_) => Ok(false),
    }
}
