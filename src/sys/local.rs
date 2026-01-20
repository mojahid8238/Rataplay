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

pub fn scan_incomplete_downloads() -> Vec<(String, String, String, String)> {
    let dir = get_download_dir();
    let mut incomplete = std::collections::HashMap::new();
    
    if let Ok(entries) = fs::read_dir(&dir) {
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
                            incomplete.insert(id.clone(), (id, title, url, format_id));
                        }
                    }
                }
            }
        }
    }
    
    incomplete.into_values().collect()
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


