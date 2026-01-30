use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use log::{info, error};

use crate::tui::components::logo::AnimationMode;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_search_limit")]
    pub search_limit: u32,
    #[serde(default = "default_playlist_limit")]
    pub playlist_limit: u32,
    #[serde(default = "default_download_directory")]
    pub download_directory: String,
    #[serde(default = "default_animation")]
    pub animation: AnimationMode,
    #[serde(default = "default_true")]
    pub show_live: bool,
    #[serde(default = "default_true")]
    pub show_playlists: bool,

    // New Fields
    #[serde(default)]
    pub executables: Executables,
    #[serde(default)]
    pub cookies: Cookies,
    #[serde(default)]
    pub logging: Logging,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Executables {
    #[serde(default = "default_false")]
    pub enabled: bool,
    pub mpv: Option<PathBuf>,
    pub ytdlp: Option<PathBuf>,
    pub ffmpeg: Option<PathBuf>,
    pub deno: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Cookies {
    pub enabled: bool,
    #[serde(default)]
    pub source: CookieSource,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum CookieSource {
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "browser")]
    Browser(String),
    #[serde(rename = "netscape")]
    Netscape(PathBuf),
    #[serde(rename = "json")]
    Json(PathBuf),
}

impl Default for CookieSource {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Logging {
    #[serde(default = "default_false")]
    pub enabled: bool,
    pub path: Option<PathBuf>,
}

impl Default for Executables {
    fn default() -> Self {
        Self {
            enabled: false,
            mpv: None,
            ytdlp: None,
            ffmpeg: None,
            deno: None,
        }
    }
}

impl Default for Cookies {
    fn default() -> Self {
        Self {
            enabled: false,
            source: CookieSource::default(),
        }
    }
}

impl Default for Logging {
    fn default() -> Self {
        Self {
            enabled: false,
            path: None,
        }
    }
}

fn default_theme() -> String {
    "Default".to_string()
}
fn default_search_limit() -> u32 {
    20
}
fn default_playlist_limit() -> u32 {
    15
}
fn default_animation() -> AnimationMode {
    AnimationMode::Glitch
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_download_directory() -> String {
    ProjectDirs::from("com", "rataplay", "rataplay")
        .and_then(|_proj_dirs| {
            directories::UserDirs::new().map(|user_dirs| {
                user_dirs
                    .video_dir()
                    .map(|p| p.join("Rataplay"))
                    .unwrap_or_else(|| user_dirs.home_dir().join("Downloads").join("Rataplay"))
            })
        })
        .unwrap_or_else(|| {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
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
            animation: default_animation(),
            show_live: default_true(),
            show_playlists: default_true(),
            executables: Executables::default(),
            cookies: Cookies::default(),
            logging: Logging::default(),
        }
    }
}

impl Config {
    pub fn get_config_path() -> PathBuf {
        ProjectDirs::from("com", "rataplay", "rataplay")
            .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
            .unwrap_or_else(|| {
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_else(|_| ".".to_string());
                Path::new(&home).join(".rataplay").join("config.toml")
            })
    }

    pub fn load() -> Result<Self> {
        let path = Self::get_config_path();

        // Ensure parent directory exists check
        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
        }

        if path.exists() {
            let content = fs::read_to_string(&path)?;
            match toml::from_str(&content) {
                Ok(config) => {
                    info!("Config loaded from {:?}", path);
                    return Ok(config);
                },
                Err(e) => {
                    error!("Failed to parse config.toml: {}", e);
                    return Err(anyhow::anyhow!("Failed to parse config: {}", e));
                }
            }
        } else {
            info!("Config file not found at {:?}, using defaults", path);
        }
        Ok(Self::default())
    }

    pub fn expand_tilde(path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        if path_str.starts_with("~/") || path_str == "~" {
            if let Some(home) = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf()) {
                if path_str == "~" {
                    return home;
                }
                return home.join(&path_str[2..]);
            }
        }
        path.to_path_buf()
    }

    pub fn get_log_path(&self) -> Result<PathBuf> {
        if let Some(path) = &self.logging.path {
            return Ok(Self::expand_tilde(path));
        }

        if let Some(dirs) = ProjectDirs::from("com", "rataplay", "rataplay") {
            #[cfg(target_os = "linux")]
            {
                // Linux: ~/.local/state/rataplay/rataplay.log (XDG State Home)
                if let Some(state) = dirs.state_dir() {
                    return Ok(state.join("rataplay.log"));
                }
            }

            #[cfg(target_os = "macos")]
            {
                // macOS: ~/Library/Logs/rataplay/rataplay.log
                if let Some(home) = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf())
                {
                    return Ok(home.join("Library/Logs/rataplay/rataplay.log"));
                }
            }

            #[cfg(target_os = "windows")]
            {
                // Windows: Local AppData/rataplay/logs/rataplay.log
                return Ok(dirs.data_local_dir().join("logs").join("rataplay.log"));
            }

            // Fallback for other Unix-like systems
            return Ok(dirs.data_local_dir().join("rataplay.log"));
        }

        anyhow::bail!("Could not determine project directories")
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::get_config_path();
        info!("Saving config to {:?}", path);
        
        // If file exists, try to preserve user comments/formatting
        if path.exists() {
            if let Ok(current_content) = fs::read_to_string(&path) {
                if let Ok(new_content) = self.update_content_preservative(&current_content) {
                    fs::write(&path, new_content)?;
                    info!("Configuration saved successfully (preservative)");
                    info!("cookies: {}", if self.cookies.enabled { "enabled" } else { "disabled" });
                    info!("logging: {}", if self.logging.enabled { "enabled" } else { "disabled" });
                    return Ok(());
                }
            }
        }

        // Fallback to full overwrite if file doesn't exist or update failed
        self.save_force()
    }

    fn update_content_preservative(&self, content: &str) -> Result<String, ()> {
        info!("Updating existing config file content...");
        let mut new_lines = Vec::new();
        let mut current_section = "".to_string();

        for line in content.lines() {
            let trimmed = line.trim();
            
            // Check for section header
            if trimmed.starts_with('[') {
                if let Some(end_idx) = trimmed.find(']') {
                    // Make sure it's not a nested array or something, strictly [section]
                    // Basic heuristic: check if it looks like a section header
                    let potential_section = trimmed[1..end_idx].trim();
                    if !potential_section.is_empty() {
                         current_section = potential_section.to_string();
                    }
                }
                new_lines.push(line.to_string());
                continue;
            }

            // Skip comments or empty lines for processing (but keep them)
            if trimmed.starts_with('#') || trimmed.is_empty() {
                new_lines.push(line.to_string());
                continue;
            }

            let mut new_line = line.to_string();

            // Try to match key-value pair
            if let Some((key_part, _val_part)) = trimmed.split_once('=') {
                let key = key_part.trim();
                
                // Handle inline comments in key part? unlikely for standard TOML keys but trimming helps
                
                if current_section.is_empty() {
                    match key {
                        "theme" => {
                            if let Ok(val) = serde_json::to_string(&self.theme) {
                                new_line = format!("theme = {}", val);
                            }
                        },
                        "search_limit" => new_line = format!("search_limit = {}", self.search_limit),
                        "playlist_limit" => new_line = format!("playlist_limit = {}", self.playlist_limit),
                        "download_directory" => {
                            if let Ok(val) = serde_json::to_string(&self.download_directory) {
                                new_line = format!("download_directory = {}", val);
                            }
                        },
                        "animation" => {
                            if let Ok(anim_val) = serde_json::to_value(self.animation) {
                                new_line = format!("animation = {}", anim_val);
                            }
                        },
                        "show_live" => new_line = format!("show_live = {}", self.show_live),
                        "show_playlists" => new_line = format!("show_playlists = {}", self.show_playlists),
                        _ => {}
                    }
                } else if current_section == "executables" {
                    if key == "enabled" {
                        new_line = format!("enabled = {}", self.executables.enabled);
                    }
                } else if current_section == "cookies" {
                    if key == "enabled" {
                        new_line = format!("enabled = {}", self.cookies.enabled);
                    }
                } else if current_section == "logging" {
                    if key == "enabled" {
                        new_line = format!("enabled = {}", self.logging.enabled);
                    } else if key == "path" {
                         if let Some(p) = &self.logging.path {
                            if let Ok(val) = serde_json::to_string(&p.to_string_lossy()) {
                                new_line = format!("path = {}", val);
                            }
                        }
                    }
                }
            }

            new_lines.push(new_line);
        }

        Ok(new_lines.join("\n"))
    }

    pub fn save_force(&self) -> anyhow::Result<()> {
        let path = Self::get_config_path();
        info!("Creating new config file at {:?}", path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Construct the full file content explicitly
        let mut content = String::from("# Rataplay Configuration\n\n");

        content.push_str("# The visual style of the application.\n");
        content.push_str("# Available themes: \"Default\", \"Dracula\", \"Matrix\", \"Cyberpunk\", \"Catppuccin\"\n");
        content.push_str(&format!("theme = {}\n\n", serde_json::to_string(&self.theme)?));

        content.push_str(
            "# The number of results to fetch per search page or \"Load More\" action.\n",
        );
        content.push_str(&format!("search_limit = {}\n\n", self.search_limit));

        content.push_str("# The maximum number of videos to load when opening a playlist.\n");
        content.push_str(&format!("playlist_limit = {}\n\n", self.playlist_limit));

        content.push_str("# The directory where videos and audio will be downloaded.\n");
        content.push_str(&format!(
            "download_directory = {}\n\n",
            serde_json::to_string(&self.download_directory)?
        ));

        content.push_str("# The animation style for the logo.\n");
        content.push_str("# Options: \"Wave\", \"Breathe\", \"Glitch\", \"Neon\", \"Static\"\n");
        content.push_str(&format!(
            "animation = \"{}\"\n\n",
            serde_json::to_value(self.animation)?
                .as_str()
                .unwrap_or("Wave")
        ));

        content.push_str("# Whether to show live streams in search results.\n");
        content.push_str(&format!("show_live = {}\n\n", self.show_live));

        content.push_str("# Whether to show playlists in search results.\n");
        content.push_str(&format!("show_playlists = {}\n\n", self.show_playlists));

        content.push_str("# --- Advanced Configuration ---\n\n");

        content.push_str("[executables]\n");
        content.push_str(&format!("enabled = {}\n", self.executables.enabled));
        content.push_str("# Absolute paths to binaries. If commented out, system PATH is used.\n");
        if let Some(p) = &self.executables.mpv {
            content.push_str(&format!("mpv = {}\n", serde_json::to_string(&p.to_string_lossy())?));
        } else {
            content.push_str("# mpv = \"/usr/bin/mpv\"\n");
        }
        if let Some(p) = &self.executables.ytdlp {
            content.push_str(&format!("ytdlp = {}\n", serde_json::to_string(&p.to_string_lossy())?));
        } else {
            content.push_str("# ytdlp = \"/usr/bin/yt-dlp\"\n");
        }
        if let Some(p) = &self.executables.ffmpeg {
            content.push_str(&format!("ffmpeg = {}\n", serde_json::to_string(&p.to_string_lossy())?));
        } else {
            content.push_str("# ffmpeg = \"/usr/bin/ffmpeg\"\n");
        }
        if let Some(p) = &self.executables.deno {
            content.push_str(&format!("deno = {}\n", serde_json::to_string(&p.to_string_lossy())?));
        } else {
            content.push_str("# deno = \"/usr/bin/deno\"\n");
        }
        content.push_str("\n");

        content.push_str("[cookies]\n");
        content.push_str("# This section configures how yt-dlp accesses cookies for protected content (like Watch Later).\n");
        content.push_str(&format!("enabled = {}\n", self.cookies.enabled));
        content.push_str("# \n");
        content.push_str("# --- How to configure ---\n");
        content.push_str("# 1. Browser: Use cookies from your installed browser.\n");
        content.push_str("# Please keep in mind that you have to install secretstorage to use this feature.\n");
        content.push_str("#    source.type = \"browser\"\n");
        content.push_str("#    source.value = \"chrome\"  # Options: chrome, firefox, edge, safari, opera, vivaldi, brave\n");
        content.push_str("# \n");
        content.push_str("# 2. Netscape: Use a Netscape-formatted cookie file.\n");
        content.push_str("#    source.type = \"netscape\"\n");
        content.push_str("#    source.value = \"/path/to/cookies.txt\"\n");
        content.push_str("# \n");
        content.push_str("# 3. JSON: Use a JSON-formatted cookie file.\n");
        content.push_str("#    source.type = \"json\"\n");
        content.push_str("#    source.value = \"/path/to/cookies.json\"\n");
        content.push_str("# ------------------------\n\n");

        match &self.cookies.source {
            CookieSource::Off => {
                content.push_str("# source.type = \"\"\n");
                content.push_str("# source.value = \"\"\n");
            }
            CookieSource::Browser(name) => {
                content.push_str("#source.type = \"browser\"\n");
                content.push_str(&format!("#source.value = {}\n", serde_json::to_string(name)?));
            }
            CookieSource::Netscape(path) => {
                content.push_str("#source.type = \"netscape\"\n");
                content.push_str(&format!("#source.value = {}\n", serde_json::to_string(&path.to_string_lossy())?));
            }
            CookieSource::Json(path) => {
                content.push_str("#source.type = \"json\"\n");
                content.push_str(&format!("#source.value = {}\n", serde_json::to_string(&path.to_string_lossy())?));
            }
        }
        content.push_str("\n");

        content.push_str("[logging]\n");
        content.push_str("# Whether to enable application logging.\n");
        content.push_str(&format!("enabled = {}\n", self.logging.enabled));
        content.push_str("# The default location of log file is:\n");
        #[cfg(target_os = "linux")]
        content.push_str("# Linux:   ~/.local/state/rataplay/rataplay.log\n");
        #[cfg(target_os = "windows")]
        content.push_str("# Windows: %LOCALAPPDATA%/rataplay/logs/rataplay.log\n");
        #[cfg(target_os = "macos")]
        content.push_str("# macOS:   ~/Library/Logs/rataplay/rataplay.log\n");
        if let Some(p) = &self.logging.path {
            content.push_str(&format!("path = {}\n", serde_json::to_string(&p.to_string_lossy())?));
        } else {
            content.push_str("# You can set a custom path for the log file.\n");
            content.push_str("# path = \"/absolute/path/to/rataplay.log\"\n");
        }

        fs::write(path, content)?;
        info!("Configuration saved successfully (new/force)");
        info!("cookies: {}", if self.cookies.enabled { "enabled" } else { "disabled" });
        Ok(())
    }
}


