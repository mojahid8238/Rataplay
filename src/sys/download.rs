use crate::model::Video;
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::process::Stdio;
use tokio::process::{Child, Command};

// Regex to parse yt-dlp output
// Example: [download]   1.5% of ~4.30MiB at  2.50MiB/s ETA 00:01
static YTDLP_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\[download\]\s+(?P<progress>\d+(?:\.\d+)?)%\s+of\s+~?(?P<size>[\d\.]+\w+)(?:\s+at\s+(?P<speed>[\d\.]+\w+/s))?(?:\s+ETA\s+(?P<eta>[\d:?]+))?",
    )
    .unwrap()
});

pub fn parse_progress(line: &str) -> Option<(f64, String, String, String)> {
    YTDLP_REGEX.captures(line).and_then(|caps: regex::Captures| {
        let progress = caps
            .name("progress")
            .and_then(|m: regex::Match| m.as_str().parse::<f64>().ok())?;
        let size = caps
            .name("size")
            .map_or(String::new(), |m: regex::Match| m.as_str().to_string());
        let speed = caps
            .name("speed")
            .map_or(String::new(), |m: regex::Match| m.as_str().to_string());
        let eta = caps
            .name("eta")
            .map_or(String::new(), |m: regex::Match| m.as_str().to_string());
        Some((progress, size, speed, eta))
    })
}

pub async fn start_download(
    video: &Video,
    format_id: &str,
) -> Result<Child> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let download_dir = std::path::Path::new(&home).join("Videos").join("Rataplay");

    if let Err(e) = tokio::fs::create_dir_all(&download_dir).await {
        anyhow::bail!("Failed to create download dir: {}", e);
    }

    let mut cmd = Command::new("yt-dlp");
    cmd.arg("-f")
        .arg(format!("{}+bestaudio/best", format_id));
    cmd.arg("-P").arg(&download_dir);
    cmd.arg("-o").arg("%(title)s - %(id)s.%(ext)s");
    cmd.arg("--newline");
    cmd.arg("--progress");
    cmd.arg(&video.url);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let child = cmd.spawn()?;
    Ok(child)
}