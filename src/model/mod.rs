use serde::{Deserialize, Serialize};

pub mod download;
pub mod local;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub url: String,
    pub duration_string: String, // e.g. "10:05"
    pub thumbnail_url: Option<String>,
    pub view_count: Option<u64>,
    pub upload_date: Option<String>,
    pub playlist_count: Option<u64>,
    pub live_status: Option<String>,

    // New fields for two-stage fetching
    #[serde(default)]
    pub is_partial: bool,
    #[serde(default)]
    pub video_type: VideoType,

    // Parent playlist info (for videos that belong to a playlist)
    #[serde(default)]
    pub parent_playlist_id: Option<String>,
    #[serde(default)]
    pub parent_playlist_url: Option<String>,
    #[serde(default)]
    pub parent_playlist_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum VideoType {
    #[default]
    Video,
    Channel,
    Playlist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFormat {
    pub format_id: String,
    pub ext: String,
    pub resolution: String, // e.g. "1920x1080"
    pub note: String,       // e.g. "video only" or "video+audio"
    pub filesize: Option<u64>,
}
