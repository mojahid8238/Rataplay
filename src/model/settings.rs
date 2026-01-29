use crate::sys::config::{Config, CookieSource};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Settings {
    pub enable_logging: bool,
    pub use_custom_paths: bool,
    pub cookie_mode: CookieMode,
    
    // Resolved paths (either from system or config depending on use_custom_paths)
    pub mpv_path: String,
    pub ytdlp_path: String,
    pub ffmpeg_path: String,
    pub deno_path: String,
    
    // Cookie details
    pub cookie_file: Option<PathBuf>,
    pub browser_name: Option<String>,
    pub log_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CookieMode {
    Off,
    File(PathBuf),
    Browser(String),
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enable_logging: false,
            use_custom_paths: false,
            cookie_mode: CookieMode::Off,
            mpv_path: "mpv".to_string(),
            ytdlp_path: "yt-dlp".to_string(),
            ffmpeg_path: "ffmpeg".to_string(),
            deno_path: "deno".to_string(),
            cookie_file: None,
            browser_name: None,
            log_path: None,
        }
    }
}

impl Settings {
    pub fn from_config(config: Config) -> Self {
        let mut settings = Self::default();
        
        settings.enable_logging = config.logging.enabled;
        settings.use_custom_paths = config.executables.enabled;

        // Populate paths from config if they exist
        if let Some(p) = config.executables.mpv {
            settings.mpv_path = p.to_string_lossy().to_string();
        }
        if let Some(p) = config.executables.ytdlp {
            settings.ytdlp_path = p.to_string_lossy().to_string();
        }
        if let Some(p) = config.executables.ffmpeg {
            settings.ffmpeg_path = p.to_string_lossy().to_string();
        }
        if let Some(p) = config.executables.deno {
            settings.deno_path = p.to_string_lossy().to_string();
        }
        
        // Cookie mode logic
        if !config.cookies.enabled {
            settings.cookie_mode = CookieMode::Off;
        } else {
            match config.cookies.source {
                CookieSource::Disabled => {
                    settings.cookie_mode = CookieMode::Off;
                }
                CookieSource::Netscape(path) | CookieSource::Json(path) => {
                    settings.cookie_mode = CookieMode::File(path.clone());
                    settings.cookie_file = Some(path);
                }
                CookieSource::Browser(name) => {
                    settings.cookie_mode = CookieMode::Browser(name.clone());
                    settings.browser_name = Some(name);
                }
            }
        }
        
        settings.log_path = config.logging.path;
        
        settings
    }
    
    pub fn update_from_config(&mut self, config: Config) {
        *self = Self::from_config(config);
    }
}
