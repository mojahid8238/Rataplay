use anyhow::Result;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
pub enum DownloadProgress {
    Started,
    Progress(f32, String), // percent, speed/eta string
    Finished,
    Error(String),
}

pub async fn download_video(
    url: String,
    format_id: String,
    tx: UnboundedSender<DownloadProgress>,
) -> Result<()> {
    let _ = tx.send(DownloadProgress::Started);

    // Resolve download directory: ~/Videos/Vivid
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let download_dir = std::path::Path::new(&home).join("Videos").join("Vivid");

    // Create directory if it doesn't exist
    if let Err(e) = tokio::fs::create_dir_all(&download_dir).await {
        let _ = tx.send(DownloadProgress::Error(format!(
            "Failed to create download dir: {}",
            e
        )));
        return Ok(());
    }

    let mut cmd = Command::new("yt-dlp");
    // Attempt to merge best audio if the user selected a video-only format.
    // "format_id+bestaudio/best" tries to download format_id + audio.
    // If format_id is already combined (has audio), yt-dlp might fail with "Video is already in target format" or similar if we force merge,
    // BUT "format_id+bestaudio" usually works fine even if format_id implies audio?
    // Actually, if format_id is 22 (720p+audio), 22+bestaudio will download 22 AND bestaudio and merge them.
    // A safer bet involves checking the format details passed, but we don't have them here easily.
    // However, the user issue is "video without audio". This overwhelmingly happens when selecting 1080p+ streams (video only).
    // The syntax `format_id+bestaudio/best` means: try format_id + bestaudio. If that fails (e.g. invalid comb), try 'best'.
    // A better syntax for "add audio if missing" isn't trivial in one flag.
    // Let's assume the user selects high quality which is usually video-only.
    // If they select a pre-merged format (like 18 or 22), adding +bestaudio is redundant but usually harmless (just double audio or re-merge).
    // Let's go with the requested fix logic.
    cmd.arg("-f").arg(format!("{}+bestaudio/best", format_id));
    cmd.arg("-P").arg(&download_dir); // Set path
    cmd.arg("-o").arg("%(title)s.%(ext)s");
    cmd.arg("--newline"); // Important for parsing
    cmd.arg("--progress");
    cmd.arg(&url);

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()); // Capture stderr too just in case

    let mut child = cmd.spawn()?;

    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();

    while reader.read_line(&mut line).await? > 0 {
        // Parse line: [download]  23.5% of 10.00MiB at 2.50MiB/s ETA 00:04
        if line.starts_with("[download]") {
            if let Some(_percent_idx) = line.find('%') {
                // Try to find the number before '%'
                let parts: Vec<&str> = line.split_whitespace().collect();
                // usually parts[1] is percentage
                for part in parts {
                    if part.contains('%') {
                        if let Ok(val) = part.trim_end_matches('%').parse::<f32>() {
                            // Send progress
                            let _ =
                                tx.send(DownloadProgress::Progress(val, line.trim().to_string()));
                            break;
                        }
                    }
                }
            }
        }
        line.clear();
    }

    let status = child.wait().await?;

    if status.success() {
        let _ = tx.send(DownloadProgress::Finished);
    } else {
        let _ = tx.send(DownloadProgress::Error(
            "Download process failed".to_string(),
        ));
    }

    Ok(())
}
