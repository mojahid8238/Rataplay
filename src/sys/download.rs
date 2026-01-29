use crate::model::Video;
use crate::model::settings::Settings;
use anyhow::Result;
use std::process::Stdio;
use tokio::process::Child;
use crate::sys::yt::build_base_command;

pub fn parse_progress(line: &str) -> Option<(f64, String, String, String)> {
    if !line.starts_with("[download]") {
        return None;
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    // Example: [download] 1.5% of ~4.30MiB at 2.50MiB/s ETA 00:01
    // idx: 0      1    2  3        4  5         6   7
    
    if parts.len() < 4 { return None; }
    
    let progress_str = parts[1].trim_end_matches('%');
    let progress = progress_str.parse::<f64>().ok()?;
    
    let size = parts[3].trim_start_matches('~').to_string();
    
    let mut speed = String::new();
    let mut eta = String::new();
    
    // Search for "at" and "ETA" as they might be missing or in different positions
    for i in 4..parts.len() {
        if parts[i] == "at" && i + 1 < parts.len() {
            speed = parts[i+1].to_string();
        } else if parts[i] == "ETA" && i + 1 < parts.len() {
            eta = parts[i+1].to_string();
        }
    }

    Some((progress, size, speed, eta))
}

pub async fn start_download(
    video: &Video,
    format_id: &str,
    download_dir: &str,
    settings: &Settings,
) -> Result<Child> {
    let download_dir = std::path::PathBuf::from(download_dir);

    if let Err(e) = tokio::fs::create_dir_all(&download_dir).await {
        anyhow::bail!("Failed to create download dir: {}", e);
    }

    let mut cmd = build_base_command(settings);
    let format_arg = if format_id == "best" {
        "bestvideo+bestaudio/best".to_string()
    } else {
        format!("{}+bestaudio/best", format_id)
    };
    
    cmd.arg("-f").arg(format_arg);
    cmd.arg("-P").arg(&download_dir);
    cmd.arg("-o").arg("%(title).150s - %(id)s.%(ext)s");
    cmd.arg("--newline");
    cmd.arg("--progress");
    cmd.arg("--write-info-json");
    cmd.arg(&video.url);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    log::info!("Starting download for video {}: {} (URL: {})", video.id, video.title, video.url);
    log::debug!("Download command: {:?}", cmd);

    let child = cmd.spawn().map_err(|e| {
        log::error!("Failed to spawn yt-dlp for download: {}", e);
        e
    })?;
    Ok(child)
}