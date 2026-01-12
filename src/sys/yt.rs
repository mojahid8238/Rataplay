use std::process::Stdio;
use anyhow::{Result, Context};
use tokio::process::Command;
use crate::model::{Video, VideoFormat};
use serde_json::Value;

pub async fn search_videos(query: &str) -> Result<Vec<Video>> {
    // Decide if it's a URL or a Search
    let is_url = query.starts_with("http://") || query.starts_with("https://");
    
    let args = if is_url {
        // If it's a URL, just get that single video's info
        vec!["--dump-json", "--no-playlist", query]
    } else {
        // If search, get top 15 results
        // "ytsearch15:<query>"
        vec![
            "--dump-json",
            "--no-playlist",
            "--default-search", "ytsearch15",
            query
        ]
    };

    let output = Command::new("yt-dlp")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()) // Hide stderr unless error
        .spawn()
        .context("Failed to spawn yt-dlp")?
        .wait_with_output()
        .await?;

    if !output.status.success() {
         let err = String::from_utf8_lossy(&output.stderr);
         anyhow::bail!("yt-dlp error: {}", err);
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut videos = Vec::new();

    // yt-dlp --dump-json prints one JSON object per line
    for line in stdout.lines() {
        if line.trim().is_empty() { continue; }
        
        if let Ok(val) = serde_json::from_str::<Value>(line) {
            let id = val["id"].as_str().unwrap_or_default().to_string();
            let title = val["title"].as_str().unwrap_or_default().to_string();
            let channel = val["uploader"].as_str().unwrap_or("Unknown").to_string();
            let duration = val["duration"].as_f64().unwrap_or(0.0);
            let thumbnail = val["thumbnail"].as_str().map(|s| s.to_string());
            let view_count = val["view_count"].as_u64();
            let upload_date = val["upload_date"].as_str().map(|s| s.to_string());

            let duration_string = format_duration(duration);

            videos.push(Video {
                id,
                title,
                channel,
                duration_string,
                thumbnail_url: thumbnail,
                view_count,
                upload_date,
            });
        }
    }

    Ok(videos)
}

pub async fn get_video_formats(url: &str) -> Result<Vec<VideoFormat>> {
    let output = Command::new("yt-dlp")
        .arg("--dump-json")
        .arg("--no-playlist")
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn yt-dlp")?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        // bail!("yt-dlp failed");
    }

    let stdout = String::from_utf8(output.stdout)?;
    // output is one JSON object
    let val: Value = serde_json::from_str(&stdout).context("Failed to parse yt-dlp JSON")?;
    
    let mut formats = Vec::new();
    if let Some(list) = val["formats"].as_array() {
        for f in list {
            let format_id = f["format_id"].as_str().unwrap_or("").to_string();
            let ext = f["ext"].as_str().unwrap_or("").to_string();
            let resolution = f["resolution"].as_str().unwrap_or("unknown").to_string();
            let format_note = f["format_note"].as_str().unwrap_or("").to_string();
            let filesize = f["filesize"].as_u64().or(f["filesize_approx"].as_u64());
            
            formats.push(VideoFormat {
                format_id,
                ext,
                resolution,
                note: format_note,
                filesize,
            });
        }
    }
    // Reverse to show best quality first effectively? Or just let user sort.
    // Usually yt-dlp sorts by worst to best.
    formats.reverse();
    
    Ok(formats)
}

fn format_duration(seconds: f64) -> String {
    let seconds = seconds as u64;
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}
