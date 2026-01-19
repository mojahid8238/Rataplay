use crate::model::local::LocalFile;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_download_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join("Videos").join("Rataplay")
}

pub fn scan_local_files() -> Vec<LocalFile> {
    let dir = get_download_dir();
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let extension = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                
                let metadata = path.metadata().ok();
                let size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let size = format_size(size_bytes);
                let is_garbage = name.ends_with(".part") || name.ends_with(".ytdl") || name.ends_with(".tmp");

                if !is_garbage {
                    files.push(LocalFile {
                        name,
                        path,
                        size,
                        extension,
                        is_garbage,
                        modified,
                    });
                }
            }
        }
    }
    // Sort by modified time descending (newest first)
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    files
}

fn format_size(bytes: u64) -> String {
    let mb = bytes as f64 / 1024.0 / 1024.0;
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{:.1} MB", mb)
    }
}

pub fn scan_incomplete_downloads() -> Vec<(String, String, String)> {
    let dir = get_download_dir();
    let mut incomplete = Vec::new();
    
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            
            // We look for .part or .ytdl files
            if name.ends_with(".part") || name.ends_with(".ytdl") {
                // The info.json filename is the base filename (before .part/.ytdl) with .info.json extension
                // Actually yt-dlp saves it as [filename].info.json
                // If the final file is "Title - ID.mp4", the info json is "Title - ID.info.json"
                // The part file is "Title - ID.mp4.part"
                
                let mut info_path = path.clone();
                let mut info_name = name.clone();
                
                // Remove .part or .ytdl
                if info_name.ends_with(".part") {
                    info_name = info_name.trim_end_matches(".part").to_string();
                } else {
                    info_name = info_name.trim_end_matches(".ytdl").to_string();
                }
                
                // Now info_name is "Title - ID.ext"
                // yt-dlp usually names info json as "Title - ID.info.json" (replacing the extension)
                // or sometimes "Title - ID.ext.info.json". 
                // Let's try to be smart.
                
                let stem = Path::new(&info_name).file_stem().unwrap_or_default().to_string_lossy().to_string();
                let info_json_name = format!("{}.info.json", stem);
                info_path.set_file_name(info_json_name);
                
                if !info_path.exists() {
                    // Try alternative: [filename].info.json
                    let alt_info_name = format!("{}.info.json", info_name);
                    info_path.set_file_name(alt_info_name);
                }

                if info_path.exists() {
                    if let Ok(content) = fs::read_to_string(info_path) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            let id = json["id"].as_str().unwrap_or("").to_string();
                            let title = json["title"].as_str().unwrap_or(&stem).to_string();
                            let url = json["webpage_url"].as_str()
                                .or_else(|| json["url"].as_str())
                                .unwrap_or("")
                                .to_string();
                            
                            if !id.is_empty() && !url.is_empty() {
                                incomplete.push((id, title, url));
                            }
                        }
                    }
                } else {
                    // Fallback for YouTube if info.json is missing but we can guess from filename
                    let base_name = name.trim_end_matches(".part").trim_end_matches(".ytdl");
                    if let Some(dash_idx) = base_name.rfind(" - ") {
                        let after_dash = &base_name[dash_idx + 3..];
                        if let Some(dot_idx) = after_dash.rfind('.') {
                            let id = &after_dash[..dot_idx];
                            let title = &base_name[..dash_idx];
                            if id.len() == 11 {
                                 incomplete.push((id.to_string(), title.to_string(), format!("https://www.youtube.com/watch?v={}", id)));
                            }
                        }
                    }
                }
            }
        }
    }
    incomplete
}

pub fn delete_file(path: &Path) -> Result<()> {
    fs::remove_file(path)?;
    Ok(())
}

pub fn cleanup_garbage() -> Result<usize> {
    let dir = get_download_dir();
    let mut count = 0;

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name.ends_with(".part") || name.ends_with(".ytdl") || name.ends_with(".tmp") || name.ends_with(".info.json") {
                    if fs::remove_file(path).is_ok() {
                        count += 1;
                    }
                }
            }
        }
    }
    Ok(count)
}


