use crate::model::Video;

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Paused,
    Canceled,
    #[allow(dead_code)]
    Finished,
    Error(String),
}

#[derive(Debug)]
pub struct DownloadTask {
    #[allow(dead_code)]
    pub id: String,
    pub title: String,
    pub video: Video,      // Store the full video object
    pub format_id: String, // Store the format ID used for download
    pub status: DownloadStatus,
    pub progress: f64, // 0.0 to 100.0
    pub speed: String,
    pub eta: String,
    pub total_size: String,
    pub pid: Option<u32>,
    pub info_json_path: Option<std::path::PathBuf>,
}

impl DownloadTask {
    pub fn new(video: Video, format_id: String) -> Self {
        Self {
            id: video.id.clone(),
            title: video.title.clone(),
            video,
            format_id,
            status: DownloadStatus::Pending,
            progress: 0.0,
            speed: String::new(),
            eta: String::new(),
            total_size: String::new(),
            pid: None,
            info_json_path: None,
        }
    }
}

#[derive(Debug)]
pub enum DownloadEvent {
    // This matches what your app is expecting: id, progress, speed, eta, total_size
    Update(String, f64, String, String, String),
    Finished(String),
    Error(String, String),
    Started(String, u32), // New variant for download start and PID
    Pause(String),        // New variant for pausing a download
    Resume(String),       // New variant for resuming a download
    Canceled(String),     // New variant for user-initiated cancellation
}
