use serde::{Deserialize, Serialize};

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
    // We might add more fields as we parse yt-dlp -j output
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFormat {
    pub format_id: String,
    pub ext: String,
    pub resolution: String, // e.g. "1920x1080"
    pub note: String,       // e.g. "video only" or "video+audio"
    pub filesize: Option<u64>,
}
