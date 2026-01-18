use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFile {
    pub name: String,
    pub path: PathBuf,
    pub size: String,
    pub extension: String,
    pub is_garbage: bool, // .part, .ytdl, .tmp
    pub modified: u64, // timestamp
}

impl LocalFile {
    pub fn is_audio(&self) -> bool {
        let audio_exts = ["mp3", "m4a", "flac", "wav", "ogg", "opus", "aac", "wma"];
        audio_exts.contains(&self.extension.to_lowercase().as_str())
    }
}
