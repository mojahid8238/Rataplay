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

    // If the query is a URL and contains known playlist identifiers (PL, UU, FL, RD, OL)
    // but wasn't caught by the explicit 'list=' or '/playlist/' check,
    // then also consider it a direct playlist URL.
    if is_url && !is_direct_playlist_url &&
       (query.contains("PL") || query.contains("UU") ||
        query.contains("FL") || query.contains("RD") ||
        query.contains("OL")) {
        is_direct_playlist_url = true;
    }

    let args = if is_url && is_direct_playlist_url {
        // This is a direct playlist URL, we want to list its contents
        vec![
            "--dump-json",
            "--flat-playlist",
            "--no-warnings",
            "--playlist-start",
            &start_str,
            "--playlist-end",
            &end_str,
            &search_query,
        ]
    } else if is_url {
        // This is a direct video URL or other single item URL
        vec![
            "--dump-json",
            "--flat-playlist", // still useful for channels/users
            "--no-warnings",
            "--playlist-end", // Limit to 1 item to get its metadata
            "1", // Always 1 for direct video URL
            &search_query,
        ]
    } else {
        vec![
            "--dump-json",
            "--flat-playlist",
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
            let id = val["id"].as_str().unwrap_or_default().to_string();
            let title = val["title"].as_str().unwrap_or_default().to_string();
            let channel = val["uploader"].as_str().unwrap_or("Unknown").to_string();
            let mut final_url = val["url"]
                .as_str()
                .or_else(|| val["webpage_url"].as_str())
                .unwrap_or("")
                .to_string();

            if final_url.is_empty() {
                final_url = id.clone();
            }

            let item_type_str = val["_type"].as_str().unwrap_or("video");
            let is_playlist_url = final_url.contains("list=") || final_url.contains("/playlist/");

            // Check if this is a real YouTube playlist ID (not just a search query)
            let playlist_id_str = val["playlist_id"].as_str().unwrap_or("");
            let is_real_playlist_id = playlist_id_str.starts_with("PL")
                || playlist_id_str.starts_with("UU")
                || playlist_id_str.starts_with("FL")
                || playlist_id_str.starts_with("RD")
                || playlist_id_str.starts_with("OL");

            // Determine video_type before thumbnail extraction
            let (video_type, _playlist_count, _duration_string, _view_count) = if item_type_str
                == "playlist"
                || item_type_str == "multi_video"
                || (item_type_str == "url" && val["ie_key"].as_str() == Some("YoutubeTab"))
                // Existing condition: if final_url contains list= or /playlist/
                || (is_playlist_url
                    && (item_type_str == "url" || item_type_str == "url_transparent"))
                // Existing condition: if yt-dlp gives a playlist_id that looks real
                || (is_real_playlist_id
                    && (item_type_str == "url" || item_type_str == "url_transparent"))
                // NEW: If final_url contains a known playlist identifier and it's a generic URL type,
                // and it wasn't already caught by is_playlist_url (which checks for 'list=' or '/playlist/')
                // or is_real_playlist_id (which checks val["playlist_id"])
                || (!is_playlist_url && !is_real_playlist_id && // This ensures we don't double count if it's already detected
                    (final_url.contains("PL") || final_url.contains("UU") ||
                     final_url.contains("FL") || final_url.contains("RD") ||
                     final_url.contains("OL")) &&
                    (item_type_str == "url" || item_type_str == "url_transparent"))
            {
                let count = val["playlist_count"]
                    .as_u64()
                    .or_else(|| val["n_entries"].as_u64());
                let duration_str = format!("{} videos", count.unwrap_or(0));
                (crate::model::VideoType::Playlist, count, duration_str, None)
            } else if item_type_str == "channel" {
                (
                    crate::model::VideoType::Channel,
                    None,
                    "N/A".to_string(),
                    None,
                )
            } else {
                // Video
                let duration = val["duration"].as_f64().unwrap_or(0.0);
                let view_count = val["view_count"].as_u64();
                (
                    crate::model::VideoType::Video,
                    None,
                    format_duration(duration),
                    view_count,
                )
            };

            // Extract thumbnail based on determined video_type
            let thumbnail: Option<String> = if video_type == crate::model::VideoType::Playlist {
                // For actual playlist entries, try playlist_thumbnails first
                val["playlist_thumbnails"]
                    .as_array()
                    .and_then(|arr| arr.iter().max_by_key(|t| t["width"].as_u64().unwrap_or(0))) // Get largest thumbnail by width
                    .and_then(|t| t["url"].as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        // Fallback: If no playlist_thumbnails or not good, try standard thumbnails
                        val["thumbnails"]
                            .as_array()
                            .and_then(|arr| arr.iter().max_by_key(|t| t["width"].as_u64().unwrap_or(0))) // Get largest thumbnail by width
                            .and_then(|t| t["url"].as_str())
                            .map(|s| s.to_string())
                    })
                    .or_else(|| {
                        // Final fallback to generic 'thumbnail' string field
                        val["thumbnail"].as_str().map(|s| s.to_string())
                    })
            } else {
                // For videos (even from playlists), use standard thumbnails array
                val["thumbnails"]
                    .as_array()
                    .and_then(|arr| arr.iter().max_by_key(|t| t["width"].as_u64().unwrap_or(0))) // Get largest thumbnail by width
                    .and_then(|t| t["url"].as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        // Final fallback to generic 'thumbnail' string field
                        val["thumbnail"].as_str().map(|s| s.to_string())
                    })
            };
            let upload_date = val["upload_date"].as_str().map(|s| s.to_string());

            let (video_type, playlist_count, duration_string, view_count) = if item_type_str
                == "playlist"
                || item_type_str == "multi_video"
                || (item_type_str == "url" && val["ie_key"].as_str() == Some("YoutubeTab"))
                || (is_playlist_url
                    && (item_type_str == "url" || item_type_str == "url_transparent"))
                || (is_real_playlist_id
                    && (item_type_str == "url" || item_type_str == "url_transparent"))
            {
                let count = val["playlist_count"]
                    .as_u64()
                    .or_else(|| val["n_entries"].as_u64());
                let duration_str = format!("{} videos", count.unwrap_or(0));
                (crate::model::VideoType::Playlist, count, duration_str, None)
            } else if item_type_str == "channel" {
                (
                    crate::model::VideoType::Channel,
                    None,
                    "N/A".to_string(),
                    None,
                )
            } else {
                // Video
                let duration = val["duration"].as_f64().unwrap_or(0.0);
                let view_count = val["view_count"].as_u64();
                (
                    crate::model::VideoType::Video,
                    None,
                    format_duration(duration),
                    view_count,
                )
            };

            if video_type == crate::model::VideoType::Playlist {
                // Prioritize canonical playlist URL using playlist_id if available
                if !playlist_id_str.is_empty() {
                    final_url = format!("https://www.youtube.com/playlist?list={}", playlist_id_str);
                } else if let Some(p_url) = val["playlist_webpage_url"].as_str() {
                    final_url = p_url.to_string();
                }
            }

            // Check if this video is part of a real playlist (not a search query)
            // This happens when browsing actual playlists
            let (parent_playlist_id, parent_playlist_url, parent_playlist_title) =
                if video_type == crate::model::VideoType::Video && is_real_playlist_id {
                    // This is a video from a real playlist
                    let playlist_url = val["playlist_webpage_url"].as_str().map(|s| s.to_string());
                    let playlist_title = val["playlist_title"].as_str().map(|s| s.to_string());
                    (
                        Some(playlist_id_str.to_string()),
                        playlist_url,
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
                upload_date,
                playlist_count,
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

    let _ = child.wait().await;
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

            // Check if this video belongs to a real YouTube playlist
            let playlist_id_str = val["playlist_id"].as_str().unwrap_or("");
            let is_real_playlist_id = playlist_id_str.starts_with("PL")
                || playlist_id_str.starts_with("UU")
                || playlist_id_str.starts_with("FL")
                || playlist_id_str.starts_with("RD")
                || playlist_id_str.starts_with("OL");

            let (parent_playlist_id, parent_playlist_url, parent_playlist_title) =
                if is_real_playlist_id {
                    let playlist_url = val["playlist_webpage_url"].as_str().map(|s| s.to_string());
                    let playlist_title = val["playlist_title"].as_str().map(|s| s.to_string());
                    (
                        Some(playlist_id_str.to_string()),
                        playlist_url,
                        playlist_title,
                    )
                } else {
                    (None, None, None)
                };

            let video = Video {
                id,
                title,
                channel,
                url,
                duration_string,
                thumbnail_url: thumbnail,
                view_count,
                upload_date,
                playlist_count: None,
                is_partial: false,
                video_type: crate::model::VideoType::Video,
                parent_playlist_id,
                parent_playlist_url,
                parent_playlist_title,
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

    child.wait().await?;

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
    let duration = val["duration"].as_f64();

    if let Some(list) = val["formats"].as_array() {
        for f in list {
            let format_id = f["format_id"].as_str().unwrap_or("").to_string();
            let ext = f["ext"].as_str().unwrap_or("").to_string();
            let vcodec = f["vcodec"].as_str().unwrap_or("none");
            let acodec = f["acodec"].as_str().unwrap_or("none");

            // Skip storyboards, images, and data-only formats
            if (vcodec == "none" && acodec == "none")
                || ext == "mhtml"
                || format_id.contains("storyboard")
            {
                continue;
            }

            let mut resolution = f["resolution"].as_str().unwrap_or("unknown").to_string();

            // If vcodec is none, it's definitely audio only
            if vcodec == "none" {
                resolution = "audio only".to_string();
            }

            let format_note = f["format_note"].as_str().unwrap_or("").to_string();

            // Skip formats that have no resolution and no note (often redundant metadata)
            if resolution == "unknown" && format_note.is_empty() {
                continue;
            }

            // Try multiple ways to get filesize
            let mut filesize = f["filesize"]
                .as_u64()
                .or_else(|| f["filesize_approx"].as_u64())
                .or_else(|| f["filesize"].as_f64().map(|v| v as u64))
                .or_else(|| f["filesize_approx"].as_f64().map(|v| v as u64));

            // If still no filesize, try to estimate from bitrate (tbr) and duration
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
            // Both are videos, sort by resolution (height)
            let get_height = |res: &str| {
                res.split('x')
                    .last()
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
            };
            get_height(b.resolution.as_str()).cmp(&get_height(a.resolution.as_str()))
        } else {
            // Both are audio, sort by filesize/bitrate (filesize as proxy)
            b.filesize.cmp(&a.filesize)
        }
    });

    Ok(formats)
}

pub async fn get_best_stream_url(url: &str) -> Result<String> {
    // We use -g to get the URL.
    // If it returns two lines (video and audio), we handle it.
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
            anyhow::bail!("No stream URL found");
        }

        // If there are two lines, it's usually video then audio.
        // mpv can play this if we join them with a space, but better yet,
        // we can return the first one and let mpv's ytdl handle the audio IF it's a simple stream.
        // HOWEVER, for DASH/HLS, -g usually returns a single manifest URL.
        // If it's two separate URLs, we join them with a special format or just take best.
        if lines.len() >= 2 {
            // This is tricky. mpv doesn't easily take two URLs on cmdline as one 'stream'.
            // But if we use 'best' instead of 'bestvideo+bestaudio', it will be slower but single.
            // Let's try to get a single combined URL if possible first.
            let fallback = Command::new("yt-dlp")
                .arg("-g")
                .arg("-f")
                .arg("best")
                .arg("--no-playlist")
                .arg(url)
                .output()
                .await?;
            if fallback.status.success() {
                let s = String::from_utf8(fallback.stdout)?;
                return Ok(s.trim().to_string());
            }
        }

        Ok(lines[0].trim().to_string())
    } else {
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
