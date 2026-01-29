use crate::model::{Video, VideoFormat};
use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

pub enum SearchResult {
    Video(Video),
    Progress(f32),
}

pub async fn search_videos_flat(
    query: &str,
    start: u32,
    end: u32,
    show_live: bool,
    show_playlists: bool,
    tx: tokio::sync::mpsc::UnboundedSender<Result<SearchResult, String>>,
) -> Result<()> {
    let is_url = query.starts_with("http://") || query.starts_with("https://");
    let start_str = start.to_string();
    let end_str = end.to_string();
    let search_query = if is_url {
        query.to_string()
    } else {
        //gives playlists metadata
        format!("https://www.youtube.com/results?search_query={}", query)
    };

    let mut is_direct_playlist_url = query.contains("list=") || query.contains("/playlist/");

    // If the query is a YouTube URL and contains known playlist identifiers
    if is_url
        && !is_direct_playlist_url
        && query.contains("youtube.com")
        && (query.contains("PL")
            || query.contains("UU")
            || query.contains("FL")
            || query.contains("RD")
            || query.contains("OL"))
    {
        is_direct_playlist_url = true;
    }

    let args = if is_url && is_direct_playlist_url {
        // This is a direct playlist URL, we want to list its contents
        vec![
            "--dump-json",
            "--flat-playlist",
            "--lazy-playlist",
            "--no-check-formats",
            "--ignore-errors",
            "--no-warnings",
            "--playlist-start",
            &start_str,
            "--playlist-end",
            &end_str,
            &search_query,
        ]
    } else if is_url {
        // This is a direct video URL or other single item URL
        // We don't use --flat-playlist here because we want full metadata for the single item
        vec![
            "--dump-json",
            "--no-check-formats",
            "--ignore-errors",
            "--no-warnings",
            &search_query,
        ]
    } else {
        vec![
            "--dump-json",
            "--flat-playlist",
            "--no-check-formats",
            "--ignore-errors",
            "--no-warnings",
            "--playlist-start", // Added to fetch a specific range
            &start_str,
            "--playlist-end",
            &end_str, // Use end to limit the number of search results
            &search_query,
        ]
    };

    let mut child = Command::new("yt-dlp")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn yt-dlp")?;

    let stdout = child.stdout.take().context("Failed to take stdout")?;
    let _stderr = child.stderr.take().context("Failed to take stderr")?; // Capture stderr
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut lines = tokio::io::AsyncBufReadExt::lines(&mut reader);

    let mut count = 0;
    let expected = if is_url && end == 1 {
        1
    } else {
        end - start + 1
    };

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(val) = serde_json::from_str::<Value>(&line) {
            // Filter out inaccessible videos
            let title = val["title"].as_str().unwrap_or_default().to_string();
            let id = val["id"].as_str().unwrap_or_default().to_string();

            // Skip if it's a known placeholder for inaccessible videos
            if title == "[Private video]"
                || title == "[Deleted video]"
                || title.is_empty()
                || id.is_empty()
            {
                continue;
            }

            // Also, skip if there's no usable URL
            let mut final_url = val["webpage_url"]
                .as_str()
                .or_else(|| val["url"].as_str())
                .unwrap_or("")
                .to_string();

            if final_url.is_empty() && !id.is_empty() {
                if let Some(original_url) = val["original_url"].as_str() {
                    final_url = original_url.to_string();
                }
            }

            if final_url.is_empty() {
                continue;
            }

            let title = val["title"].as_str().unwrap_or_default().to_string();
            let channel = val["uploader"]
                .as_str()
                .or_else(|| val["uploader_id"].as_str())
                .or_else(|| val["webpage_url_domain"].as_str())
                .unwrap_or("Unknown")
                .to_string();

            let item_type_str = val["_type"].as_str().unwrap_or("video");

            // For entries in a playlist, final_url might contain 'list=' but we want to check if it's primarily a video
            // Check if this is a real YouTube playlist ID (not just a search query)
            let playlist_id_str = val["playlist_id"].as_str().unwrap_or("");
            let is_real_playlist_id = playlist_id_str.starts_with("PL")
                || playlist_id_str.starts_with("UU")
                || playlist_id_str.starts_with("FL")
                || playlist_id_str.starts_with("RD")
                || playlist_id_str.starts_with("OL");

            // Determine video_type before thumbnail extraction
            let (video_type, playlist_count, duration_string, view_count, concurrent_view_count) =
                if item_type_str == "playlist" || item_type_str == "multi_video" {
                    let count = val["playlist_count"]
                        .as_u64()
                        .or_else(|| val["n_entries"].as_u64());
                    let duration_str = format!("{} videos", count.unwrap_or(0));
                    (
                        crate::model::VideoType::Playlist,
                        count,
                        duration_str,
                        None,
                        None,
                    )
                } else if item_type_str == "channel" {
                    (
                        crate::model::VideoType::Channel,
                        None,
                        "N/A".to_string(),
                        None,
                        None,
                    )
                } else if (item_type_str == "url" || item_type_str == "url_transparent")
                    && val["ie_key"].as_str() == Some("YoutubeTab")
                    && (final_url.contains("list=") || final_url.contains("/playlist/"))
                {
                    // This handles playlist references in search results
                    let count = val["playlist_count"]
                        .as_u64()
                        .or_else(|| val["n_entries"].as_u64());
                    let duration_str = format!("{} videos", count.unwrap_or(0));
                    (
                        crate::model::VideoType::Playlist,
                        count,
                        duration_str,
                        None,
                        None,
                    )
                } else {
                    // Default to Video
                    let duration = val["duration"].as_f64().unwrap_or(0.0);
                    let view_count = val["view_count"].as_u64();
                    let concurrent_view_count = val["concurrent_view_count"].as_u64();
                    (
                        crate::model::VideoType::Video,
                        None,
                        format_duration(duration),
                        view_count,
                        concurrent_view_count,
                    )
                };

            // Extract live_status before thumbnail extraction for easier access
            let live_status = val["live_status"].as_str().map(|s| s.to_string());
            let is_live_bool = val["is_live"].as_bool().unwrap_or(false);

            if live_status.as_deref() == Some("is_upcoming") {
                continue;
            }

            // If it's a direct URL (not a search), we bypass filters because the user
            // specifically requested this item/playlist.
            if !is_url {
                if !show_live && (live_status.as_deref() == Some("is_live") || is_live_bool) {
                    continue;
                }

                if !show_playlists && video_type == crate::model::VideoType::Playlist {
                    continue;
                }
            }

            // Extract thumbnail based on determined video_type
            let thumbnail: Option<String> = if video_type == crate::model::VideoType::Playlist {
                // For actual playlist entries, try playlist_thumbnails first
                val["playlist_thumbnails"]
                    .as_array()
                    .and_then(|arr| arr.last())
                    .and_then(|t| t["url"].as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        val["thumbnails"]
                            .as_array()
                            .and_then(|arr| arr.last())
                            .and_then(|t| t["url"].as_str())
                            .map(|s| s.to_string())
                    })
                    .or_else(|| val["thumbnail"].as_str().map(|s| s.to_string()))
            } else {
                // For videos, try thumbnails array first, then fallback to single thumbnail field
                val["thumbnails"]
                    .as_array()
                    .and_then(|arr| arr.last())
                    .and_then(|t| t["url"].as_str())
                    .map(|s| s.to_string())
                    .or_else(|| val["thumbnail"].as_str().map(|s| s.to_string()))
                    .or_else(|| val["url"].as_str().map(|s| s.to_string())) // Last ditch effort of existing logic
                    .or_else(|| {
                        // New Fallback: Construct URL from ID
                        if !id.is_empty() {
                            Some(format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", id))
                        } else {
                            None
                        }
                    })
            };
            let upload_date = val["upload_date"].as_str().map(|s| s.to_string());

            if video_type == crate::model::VideoType::Playlist {
                // Always construct canonical URL for real playlists
                if is_real_playlist_id {
                    final_url =
                        format!("https://www.youtube.com/playlist?list={}", playlist_id_str);
                } else if let Some(p_url) = val["playlist_webpage_url"].as_str() {
                    // Only use provided URL if it's not a search result page
                    if !p_url.contains("results?search_query") {
                        final_url = p_url.to_string();
                    }
                }
            }

            // Check if this video is part of a real playlist (not a search query)
            // This happens when browsing actual playlists
            let (parent_playlist_id, parent_playlist_url, parent_playlist_title) =
                if video_type == crate::model::VideoType::Video && is_real_playlist_id {
                    // This is a video from a real playlist
                    let playlist_url = format!("https://www.youtube.com/playlist?list={}", playlist_id_str);
                    let playlist_title = val["playlist_title"].as_str().map(|s| s.to_string());
                    (
                        Some(playlist_id_str.to_string()),
                        Some(playlist_url),
                        playlist_title,
                    )
                } else {
                    (None, None, None)
                };

            let video = Video {
                id,
                title,
                channel,
                url: final_url,
                duration_string,
                thumbnail_url: thumbnail,
                view_count,
                concurrent_view_count,
                upload_date,
                playlist_count,
                live_status,
                is_partial: true,
                video_type,
                parent_playlist_id,
                parent_playlist_url,
                parent_playlist_title,
            };

            count += 1;
            let progress = (count as f32 / expected as f32).min(1.0);
            let _ = tx.send(Ok(SearchResult::Video(video)));
            // We removed progress bar from UI plan, but keeping the event for now as App handles it
            let _ = tx.send(Ok(SearchResult::Progress(progress)));
        }
    }

    let output = child.wait_with_output().await?; // Await child and capture output

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        let _ = tx.send(Err(format!("yt-dlp error: {}", err_msg)));
        anyhow::bail!("yt-dlp exited with error: {}", err_msg);
    }

    let _ = tx.send(Ok(SearchResult::Progress(1.0)));
    Ok(())
}

pub async fn resolve_video_details(
    items: Vec<String>,
    tx: tokio::sync::mpsc::UnboundedSender<Result<Video, String>>,
) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }

    let mut args = vec!["--dump-json", "--no-warnings"];
    for item in &items {
        args.push(item);
    }

    let mut child = Command::new("yt-dlp")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn yt-dlp for details")?;

    let stdout = child.stdout.take().context("Failed to take stdout")?;
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut lines = tokio::io::AsyncBufReadExt::lines(&mut reader);

    let mut collected_output = Vec::new(); // Collect output lines to re-process if needed
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        collected_output.push(line);
    }

    let output = child.wait_with_output().await?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        let _ = tx.send(Err(format!("yt-dlp error: {}", err_msg)));
        anyhow::bail!("yt-dlp exited with error: {}", err_msg);
    }

    // Now process the collected output
    for line in collected_output {
        if let Ok(val) = serde_json::from_str::<Value>(&line) {
            let id = val["id"].as_str().unwrap_or_default().to_string();
            let title = val["title"].as_str().unwrap_or_default().to_string();
            let channel = val["uploader"]
                .as_str()
                .or_else(|| val["uploader_id"].as_str())
                .or_else(|| val["webpage_url_domain"].as_str())
                .unwrap_or("Unknown")
                .to_string();

            let url = val["webpage_url"]
                .as_str()
                .or_else(|| val["url"].as_str())
                .or_else(|| val["original_url"].as_str())
                .unwrap_or_default()
                .to_string();
            let duration = val["duration"].as_f64().unwrap_or(0.0);
            let thumbnail = val["thumbnail"].as_str().map(|s| s.to_string());
            let view_count = val["view_count"].as_u64();
            let concurrent_view_count = val["concurrent_view_count"].as_u64();
            let upload_date = val["upload_date"].as_str().map(|s| s.to_string());

            let duration_string = format_duration(duration);

            // Check if this video belongs to a real YouTube playlist
            let playlist_id_str = val["playlist_id"].as_str().unwrap_or("");
            let is_real_playlist_id = playlist_id_str.starts_with("PL")
                || playlist_id_str.starts_with("UU")
                || playlist_id_str.starts_with("FL")
                || playlist_id_str.starts_with("RD")
                || playlist_id_str.starts_with("OL");

            let (parent_playlist_id, parent_playlist_url, parent_playlist_title) =
                if is_real_playlist_id {
                    let playlist_url = format!("https://www.youtube.com/playlist?list={}", playlist_id_str);
                    let playlist_title = val["playlist_title"].as_str().map(|s| s.to_string());
                    (
                        Some(playlist_id_str.to_string()),
                        Some(playlist_url),
                        playlist_title,
                    )
                } else {
                    (None, None, None)
                };

            let live_status = val["live_status"].as_str().map(|s| s.to_string());

            let video = Video {
                id,
                title,
                channel,
                url,
                duration_string,
                thumbnail_url: thumbnail,
                view_count,
                concurrent_view_count,
                upload_date,
                playlist_count: None,
                is_partial: false,
                video_type: crate::model::VideoType::Video,
                parent_playlist_id,
                parent_playlist_url,
                parent_playlist_title,
                live_status,
            };
            if tx.send(Ok(video)).is_err() {
                // Receiver dropped, so we can stop.
                break;
            }
        } else {
            // Forward errors
            if tx
                .send(Err(format!("Failed to parse yt-dlp JSON: {}", line)))
                .is_err()
            {
                break;
            }
        }
    }

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
        let _err = String::from_utf8_lossy(&output.stderr);
    }

    let stdout = String::from_utf8(output.stdout)?;
    let val: Value = serde_json::from_str(&stdout).context("Failed to parse yt-dlp JSON")?;

    let mut formats = Vec::new();
    let duration = val["duration"].as_f64();

    if let Some(list) = val["formats"].as_array() {
        for f in list {
            let format_id = f["format_id"].as_str().unwrap_or("").to_string();
            let ext = f["ext"].as_str().unwrap_or("").to_string();
            let vcodec = f["vcodec"].as_str().unwrap_or("none");
            let acodec = f["acodec"].as_str().unwrap_or("none");

            // Skip storyboards, images
            if ext == "mhtml" || format_id.contains("storyboard") {
                continue;
            }

            if vcodec == "none" && acodec == "none" && !format_id.contains("combined") {
                // Potentially metadata only, but let's be careful.
                // continue;
            }

            let mut resolution = f["resolution"]
                .as_str()
                .or_else(|| f["format_note"].as_str())
                .unwrap_or("unknown")
                .to_string();

            if vcodec == "none" && acodec != "none" {
                resolution = "audio only".to_string();
            }

            let format_note = f["format_note"].as_str().unwrap_or("").to_string();

            let mut filesize = f["filesize"]
                .as_u64()
                .or_else(|| f["filesize_approx"].as_u64())
                .or_else(|| f["filesize"].as_f64().map(|v| v as u64))
                .or_else(|| f["filesize_approx"].as_f64().map(|v| v as u64));

            if filesize.is_none() {
                if let (Some(tbr), Some(dur)) = (f["tbr"].as_f64(), duration) {
                    filesize = Some(((tbr * 1000.0 / 8.0) * dur) as u64);
                }
            }

            formats.push(VideoFormat {
                format_id,
                ext,
                resolution,
                note: format_note,
                filesize,
            });
        }
    }

    // Sort: Videos first (highest resolution), then audio only
    formats.sort_by(|a, b| {
        let a_is_audio = a.resolution == "audio only" || a.note.contains("audio only");
        let b_is_audio = b.resolution == "audio only" || b.note.contains("audio only");

        if a_is_audio && !b_is_audio {
            std::cmp::Ordering::Greater
        } else if !a_is_audio && b_is_audio {
            std::cmp::Ordering::Less
        } else if !a_is_audio && !b_is_audio {
            let get_height = |res: &str| {
                res.split('x')
                    .last()
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
            };
            get_height(b.resolution.as_str()).cmp(&get_height(a.resolution.as_str()))
        } else {
            b.filesize.cmp(&a.filesize)
        }
    });

    Ok(formats)
}

pub async fn get_best_stream_url(url: &str) -> Result<String> {
    // We use -g to get the URL.
    let output = Command::new("yt-dlp")
        .arg("-g")
        .arg("-f")
        .arg("bestvideo+bestaudio/best")
        .arg("--no-playlist")
        .arg(url)
        .output()
        .await?;

    if output.status.success() {
        let s = String::from_utf8(output.stdout)?;
        let lines: Vec<&str> = s.lines().collect();
        if lines.is_empty() {
            // Fallback to simple 'best'
            let fallback = Command::new("yt-dlp")
                .arg("-g")
                .arg("-f")
                .arg("best")
                .arg("--no-playlist")
                .arg(url)
                .output()
                .await?;
            if fallback.status.success() {
                return Ok(String::from_utf8(fallback.stdout)?.trim().to_string());
            }
            anyhow::bail!("No stream URL found");
        }

        if lines.len() >= 2 {
            let fallback = Command::new("yt-dlp")
                .arg("-g")
                .arg("-f")
                .arg("best")
                .arg("--no-playlist")
                .arg(url)
                .output()
                .await?;
            if fallback.status.success() {
                let res = String::from_utf8(fallback.stdout)?.trim().to_string();
                if !res.is_empty() {
                    return Ok(res);
                }
            }
        }

        Ok(lines[0].trim().to_string())
    } else {
        // Final fallback to 'best'
        let fallback = Command::new("yt-dlp")
            .arg("-g")
            .arg("-f")
            .arg("best")
            .arg("--no-playlist")
            .arg(url)
            .output()
            .await?;

        if fallback.status.success() {
            return Ok(String::from_utf8(fallback.stdout)?.trim().to_string());
        }

        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "yt-dlp error: {}",
            err.lines().next().unwrap_or("Unknown error")
        )
    }
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
