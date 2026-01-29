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
    Netscape(PathBuf),
    Json(PathBuf),
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

        // If any paths are provided, we implicitly "use custom paths" for those items,
        // but we still honor the global enabled flag for whether we *save* them as enabled.
        // However, the user wants them to be "first priority" if they are there.
        settings.use_custom_paths = config.executables.enabled;

        // Populate paths from config regardless of enabled status (to preserve them)
        if let Some(p) = config.executables.mpv {
            settings.mpv_path = Config::expand_tilde(&p).to_string_lossy().to_string();
        }
        if let Some(p) = config.executables.ytdlp {
            settings.ytdlp_path = Config::expand_tilde(&p).to_string_lossy().to_string();
        }
        if let Some(p) = config.executables.ffmpeg {
            settings.ffmpeg_path = Config::expand_tilde(&p).to_string_lossy().to_string();
        }
        if let Some(p) = config.executables.deno {
            settings.deno_path = Config::expand_tilde(&p).to_string_lossy().to_string();
        }

        // Set current operational mode and populate details
        if !config.cookies.enabled {
            settings.cookie_mode = CookieMode::Off;

            // Still populate details so they are preserved in memory/TUI
            match &config.cookies.source {
                CookieSource::Netscape(path) | CookieSource::Json(path) => {
                    settings.cookie_file = Some(path.clone());
                }
                CookieSource::Browser(name) => {
                    settings.browser_name = Some(name.clone());
                }
                CookieSource::Off => {}
            }
        } else {
            match &config.cookies.source {
                CookieSource::Off => {
                    settings.cookie_mode = CookieMode::Off;
                }
                CookieSource::Netscape(path) => {
                    settings.cookie_file = Some(path.clone());
                    settings.cookie_mode = CookieMode::Netscape(path.clone());
                }
                CookieSource::Json(path) => {
                    settings.cookie_file = Some(path.clone());
                    settings.cookie_mode = CookieMode::Json(path.clone());
                }
                CookieSource::Browser(name) => {
                    settings.browser_name = Some(name.clone());
                    settings.cookie_mode = CookieMode::Browser(name.clone());
                }
            }
        }

        settings.log_path = config.logging.path;

        settings
    }

    pub fn update_from_config(&mut self, config: Config) {
        *self = Self::from_config(config);
    }

    // Helper methods to get effective command
    pub fn mpv_cmd(&self) -> &str {
        if self.use_custom_paths {
            &self.mpv_path
        } else {
            "mpv"
        }
    }
    pub fn ytdlp_cmd(&self) -> &str {
        if self.use_custom_paths {
            &self.ytdlp_path
        } else {
            "yt-dlp"
        }
    }
    pub fn ffmpeg_cmd(&self) -> &str {
        if self.use_custom_paths {
            &self.ffmpeg_path
        } else {
            "ffmpeg"
        }
    }
    pub fn deno_cmd(&self) -> &str {
        if self.use_custom_paths {
            &self.deno_path
        } else {
            "deno"
        }
    }
}
