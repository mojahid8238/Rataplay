use crate::model::local::LocalFile;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub fn resolve_path(path_str: &str) -> PathBuf {
    let path = if path_str.starts_with("~/") {
        let home = directories::UserDirs::new()
            .map(|u| u.home_dir().to_path_buf())
            .or_else(|| std::env::var("HOME").ok().map(PathBuf::from))
            .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        home.join(&path_str[2..])
    } else {
        // Expand environment variables like %VAR% or $VAR
        let mut expanded = path_str.to_string();
        
        // Handle Windows style %VAR%
        if cfg!(windows) {
            for (key, value) in std::env::vars() {
                let pattern = format!("%{}%", key);
                if expanded.contains(&pattern) {
                    expanded = expanded.replace(&pattern, &value);
                }
            }
        }
        
        PathBuf::from(expanded)
    };
    path
}

pub fn scan_local_files(dir: &Path) -> Vec<LocalFile> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
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
                let is_garbage = name.ends_with(".part") || 
                                name.ends_with(".ytdl") || 
                                name.ends_with(".tmp") || 
                                name.ends_with(".info.json") ||
                                name.ends_with(".json");

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

pub fn scan_download_tasks(dir: &Path) -> Vec<(crate::model::Video, String, crate::model::download::DownloadStatus, PathBuf)> {
    let mut tasks = Vec::new();
    
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            
            if name.ends_with(".info.json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        let id = json["id"].as_str().unwrap_or("").to_string();
                        let title = json["title"].as_str().unwrap_or("").to_string();
                        let url = json["webpage_url"].as_str()
                            .or_else(|| json["url"].as_str())
                            .unwrap_or("")
                            .to_string();
                        let format_id = json["format_id"].as_str().unwrap_or("best").to_string();
                        
                        if !id.is_empty() && !url.is_empty() {
                            let mut video = crate::model::Video::default();
                            video.id = id.clone();
                            video.title = title.clone();
                            video.url = url.clone();
                            
                            let base_path_str = path.to_string_lossy();
                            let base_path = base_path_str.strip_suffix(".info.json").unwrap_or(&base_path_str);
                            
                            let mut status = crate::model::download::DownloadStatus::Finished;
                            
                             if let Some(filename) = json["_filename"].as_str() {
                                 let part_path = PathBuf::from(format!("{}.part", filename));
                                 if part_path.exists() {
                                     status = crate::model::download::DownloadStatus::Canceled;
                                 }
                             } else {
                                let extensions = ["mp4", "mkv", "webm", "mp3", "m4a", "opus"];
                                for ext in extensions {
                                    let part_path = PathBuf::from(format!("{}.{}.part", base_path, ext));
                                     if part_path.exists() {
                                         status = crate::model::download::DownloadStatus::Canceled;
                                         break;
                                     }
                                     let part_path_2 = PathBuf::from(format!("{}.part", base_path));
                                     if part_path_2.exists() {
                                          status = crate::model::download::DownloadStatus::Canceled;
                                          break;
                                     }
                                }
                             }
                            
                            tasks.push((video, format_id, status, path.clone()));
                        }
                    }
                }
            }
        }
    }
    tasks
}

pub fn delete_task_files(info_json: &Path) -> Result<()> {
    if !info_json.exists() {
        return Ok(());
    }

    // Capture the base path before deleting the json
    let base_path_str = info_json.to_string_lossy().to_string();
    let base_path = base_path_str.strip_suffix(".info.json").unwrap_or(&base_path_str);
    
    // 1. Delete the info.json
    let _ = fs::remove_file(info_json);
    
    // 2. Try to find and delete the .part file
    // Heuristic matches what scan_download_tasks uses
    let extensions = ["mp4", "mkv", "webm", "mp3", "m4a", "opus"];
    for ext in extensions {
        let part_path = PathBuf::from(format!("{}.{}.part", base_path, ext));
        if part_path.exists() {
            let _ = fs::remove_file(part_path);
            break;
        }
        let part_path_2 = PathBuf::from(format!("{}.part", base_path));
        if part_path_2.exists() {
            let _ = fs::remove_file(part_path_2);
            break;
        }
    }

    Ok(())
}

pub fn delete_file(path: &Path) -> Result<()> {
    fs::remove_file(path)?;
    Ok(())
}

pub fn cleanup_garbage(dir: &Path) -> Result<usize> {
    let mut count = 0;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                // We keep .info.json now as it represents persistent metadata
                if name.ends_with(".part") || name.ends_with(".ytdl") || name.ends_with(".tmp") {
                    if fs::remove_file(path).is_ok() {
                        count += 1;
                    }
                }
            }
        }
    }
    Ok(count)
}


