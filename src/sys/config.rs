use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_search_limit")]
    pub search_limit: u32,
    #[serde(default = "default_playlist_limit")]
    pub playlist_limit: u32,
    #[serde(default = "default_download_directory")]
    pub download_directory: String,
}

fn default_theme() -> String { "Default".to_string() }
fn default_search_limit() -> u32 { 20 }
fn default_playlist_limit() -> u32 { 100 }
fn default_download_directory() -> String {
    ProjectDirs::from("com", "rataplay", "rataplay")
        .and_then(|_proj_dirs| {
            directories::UserDirs::new().map(|user_dirs| {
                user_dirs.video_dir()
                    .map(|p| p.join("Rataplay"))
                    .unwrap_or_else(|| user_dirs.home_dir().join("Downloads").join("Rataplay"))
            })
        })
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".to_string());
            Path::new(&home).join("Videos").join("Rataplay")
        })
        .to_string_lossy()
        .to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self { 
            theme: default_theme(),
            search_limit: default_search_limit(),
            playlist_limit: default_playlist_limit(),
            download_directory: default_download_directory(),
        }
    }
}

impl Config {
    pub fn get_config_path() -> PathBuf {
        ProjectDirs::from("com", "rataplay", "rataplay")
            .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".to_string());
                Path::new(&home).join(".rataplay").join("config.toml")
            })
    }

    pub fn load() -> Self {
        let path = Self::get_config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::get_config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Construct the full file content explicitly
        let mut content = String::from("# Rataplay Configuration\n\n");
        
        content.push_str("# The visual style of the application.\n");
        content.push_str("# Available themes: \"Default\", \"Dracula\", \"Matrix\", \"Cyberpunk\", \"Catppuccin\"\n");
        content.push_str(&format!("theme = \"{}\"\n\n", self.theme));
        
        content.push_str("# The number of results to fetch per search page or \"Load More\" action.\n");
        content.push_str(&format!("search_limit = {}\n\n", self.search_limit));
        
        content.push_str("# The maximum number of videos to load when opening a playlist.\n");
        content.push_str(&format!("playlist_limit = {}\n\n", self.playlist_limit));
        
        content.push_str("# The directory where videos and audio will be downloaded.\n");
        content.push_str(&format!("download_directory = \"{}\"\n", self.download_directory));

        fs::write(path, content)?;
        Ok(())
    }
}
