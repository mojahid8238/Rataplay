use crate::model::{Video, VideoFormat};
use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

pub enum SearchResult {
    Video(Video),
    Progress(f32),
}

pub async fn search_videos(
    query: &str,
    start: u32,
    end: u32,
    tx: tokio::sync::mpsc::UnboundedSender<Result<SearchResult, String>>,
) -> Result<()> {
    let is_url = query.starts_with("http://") || query.starts_with("https://");
    let start_str = start.to_string();
    let search_query = if is_url {
        query.to_string()
    } else {
        format!("ytsearch{}:{}", end, query)
    };

    let args = if is_url {
        // If it's a URL, just get that single video's info
        vec!["--dump-json", "--no-playlist", &search_query]
    } else {
        // If search, get results in the specified range
        vec!["--dump-json", "--playlist-start", &start_str, &search_query]
    };

    let mut child = Command::new("yt-dlp")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn yt-dlp")?;

    let stdout = child.stdout.take().context("Failed to take stdout")?;
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut lines = tokio::io::AsyncBufReadExt::lines(&mut reader);

    let mut count = 0;
    let expected = if is_url { 1 } else { end - start + 1 };

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(val) = serde_json::from_str::<Value>(&line) {
            let id = val["id"].as_str().unwrap_or_default().to_string();
            let title = val["title"].as_str().unwrap_or_default().to_string();
            let channel = val["uploader"].as_str().unwrap_or("Unknown").to_string();
            let url = val["webpage_url"].as_str().unwrap_or_default().to_string();
            let duration = val["duration"].as_f64().unwrap_or(0.0);
            let thumbnail = val["thumbnail"].as_str().map(|s| s.to_string());
            let view_count = val["view_count"].as_u64();
            let upload_date = val["upload_date"].as_str().map(|s| s.to_string());

            let duration_string = format_duration(duration);

            let video = Video {
                id,
                title,
                channel,
                url,
                duration_string,
                thumbnail_url: thumbnail,
                view_count,
                upload_date,
            };

            count += 1;
            let progress = (count as f32 / expected as f32).min(1.0);
            let _ = tx.send(Ok(SearchResult::Video(video)));
            let _ = tx.send(Ok(SearchResult::Progress(progress)));
        }
    }

    let _ = child.wait().await;
    let _ = tx.send(Ok(SearchResult::Progress(1.0)));
    Ok(())
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
    formats.reverse();
    Ok(formats)
}

pub fn format_duration(seconds: f64) -> String {
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
