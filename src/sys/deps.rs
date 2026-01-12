use std::process::Command;
use anyhow::{Result, bail, Context};
use chrono::{NaiveDate, Utc};
use regex::Regex;

pub struct DependencyStatus {
    pub yt_dlp_version: String,
    pub yt_dlp_up_to_date: bool,
    pub mpv_installed: bool,
}

pub fn check_dependencies() -> Result<DependencyStatus> {
    let (version, up_to_date) = check_yt_dlp()?;
    let mpv = check_mpv()?;

    Ok(DependencyStatus {
        yt_dlp_version: version,
        yt_dlp_up_to_date: up_to_date,
        mpv_installed: mpv,
    })
}

fn check_yt_dlp() -> Result<(String, bool)> {
    let output = Command::new("yt-dlp")
        .arg("--version")
        .output()
        .context("Failed to execute yt-dlp. Is it installed and in your PATH?")?;

    if !output.status.success() {
        bail!("yt-dlp command failed with status: {}", output.status);
    }

    let version_str = String::from_utf8(output.stdout)?
        .trim()
        .to_string();
    
    // yt-dlp version format is typically YYYY.MM.DD
    // Verification
    let re = Regex::new(r"(\d{4})\.(\d{2})\.(\d{2})").unwrap();
    let up_to_date = if let Some(caps) = re.captures(&version_str) {
        let year: i32 = caps[1].parse()?;
        let month: u32 = caps[2].parse()?;
        let day: u32 = caps[3].parse()?;
        
        let version_date = NaiveDate::from_ymd_opt(year, month, day)
            .context("Invalid date in yt-dlp version")?;
        
        let now = Utc::now().date_naive();
        let days_diff = (now - version_date).num_days();
        
        days_diff <= 14 // Consider outdated if older than 14 days
    } else {
        // If we can't parse it, assume it's possibly weird custom build but warn?
        // For safety, let's say false so user checks it.
        false
    };

    Ok((version_str, up_to_date))
}

fn check_mpv() -> Result<bool> {
    // Just check availability
    let output = Command::new("mpv")
        .arg("--version")
        .output();
        
    match output {
        Ok(o) => Ok(o.status.success()),
        Err(_) => Ok(false),
    }
}
