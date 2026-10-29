#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertTarget {
    Mp4,
    Mkv,
    Webm,
    Mp3,
    M4a,
    Opus,
    Flac,
    Wav,
}

impl ConvertTarget {
    pub const ALL: [ConvertTarget; 8] = [
        ConvertTarget::Mp4,
        ConvertTarget::Mkv,
        ConvertTarget::Webm,
        ConvertTarget::Mp3,
        ConvertTarget::M4a,
        ConvertTarget::Opus,
        ConvertTarget::Flac,
        ConvertTarget::Wav,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ConvertTarget::Mp4 => "MP4",
            ConvertTarget::Mkv => "MKV",
            ConvertTarget::Webm => "WEBM",
            ConvertTarget::Mp3 => "MP3",
            ConvertTarget::M4a => "M4A",
            ConvertTarget::Opus => "OPUS",
            ConvertTarget::Flac => "FLAC",
            ConvertTarget::Wav => "WAV",
        }
    }

    pub fn description(&self) -> &'static str {
        if self.is_audio() { "audio" } else { "video" }
    }

    pub fn ext(&self) -> &'static str {
        match self {
            ConvertTarget::Mp4 => "mp4",
            ConvertTarget::Mkv => "mkv",
            ConvertTarget::Webm => "webm",
            ConvertTarget::Mp3 => "mp3",
            ConvertTarget::M4a => "m4a",
            ConvertTarget::Opus => "opus",
            ConvertTarget::Flac => "flac",
            ConvertTarget::Wav => "wav",
        }
    }

    pub fn is_audio(&self) -> bool {
        matches!(
            self,
            ConvertTarget::Mp3
                | ConvertTarget::M4a
                | ConvertTarget::Opus
                | ConvertTarget::Flac
                | ConvertTarget::Wav
        )
    }

    /// Pseudo format id used by the download pipeline to request a conversion.
    pub fn format_id(&self) -> String {
        format!("convert:{}", self.ext())
    }

    pub fn from_format_id(format_id: &str) -> Option<Self> {
        let target = format_id.strip_prefix("convert:")?;
        Self::ALL.iter().copied().find(|t| t.ext() == target)
    }

    /// FFmpeg arguments used to convert an existing local file to this target.
    pub fn ffmpeg_args(&self) -> &'static [&'static str] {
        match self {
            ConvertTarget::Mp4 => &[
                "-c:v",
                "libx264",
                "-preset",
                "medium",
                "-crf",
                "23",
                "-c:a",
                "aac",
                "-b:a",
                "192k",
                "-movflags",
                "+faststart",
            ],
            ConvertTarget::Mkv => &["-c", "copy"],
            ConvertTarget::Webm => &[
                "-c:v",
                "libvpx-vp9",
                "-crf",
                "32",
                "-b:v",
                "0",
                "-c:a",
                "libopus",
                "-b:a",
                "128k",
            ],
            ConvertTarget::Mp3 => &["-vn", "-c:a", "libmp3lame", "-q:a", "2"],
            ConvertTarget::M4a => &["-vn", "-c:a", "aac", "-b:a", "192k"],
            ConvertTarget::Opus => &["-vn", "-c:a", "libopus", "-b:a", "128k"],
            ConvertTarget::Flac => &["-vn", "-c:a", "flac"],
            ConvertTarget::Wav => &["-vn", "-c:a", "pcm_s16le"],
        }
    }
}
