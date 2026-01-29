use anyhow::Result;
use std::process::Stdio;
use tokio::process::{Child, Command};
use crate::model::settings::Settings;

fn is_audio_path(path: &str) -> bool {
    let audio_exts = [".mp3", ".m4a", ".flac", ".wav", ".ogg", ".opus", ".aac", ".wma"];
    let path_lower = path.to_lowercase();
    audio_exts.iter().any(|ext| path_lower.ends_with(ext))
}

pub fn play_video(url: &str, in_terminal: bool, user_agent: Option<&str>, settings: &Settings) -> Result<Child> {
    let mut cmd = Command::new(settings.mpv_cmd());
    cmd.kill_on_drop(true);

    if let Some(ua) = user_agent {
        cmd.arg(format!("--user-agent={}", ua));
        // Also set ytdl=no to avoid double extraction which often causes 403
        cmd.arg("--ytdl=no");
    }

    // Common IPC setup
    // Common IPC setup
    let socket_path = if cfg!(windows) {
        format!(r"\\.\pipe\rataplay-mpv-{}", std::process::id())
    } else {
        format!("/tmp/rataplay-mpv-{}.sock", std::process::id())
    };
    cmd.arg(format!("--input-ipc-server={}", socket_path));

    if in_terminal {
        cmd.arg("--vo=tct");
        cmd.arg("--really-quiet");
        
        if is_audio_path(url) {
            // For audio in terminal, show a simple visualizer or just the OSC
            cmd.arg("--force-window=no");
        }

        // Buffering and speed flags
        cmd.arg("--cache=yes");
        cmd.arg("--cache-secs=2");
        cmd.arg("--demuxer-max-bytes=10M");
        cmd.arg("--demuxer-readahead-secs=2");
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
    } else {
        // Detached / background
        cmd.arg("--idle=yes");
        cmd.arg("--force-window=yes");
        cmd.arg("--fs");

        if is_audio_path(url) {
            // Add visualizer for audio files in external window
            cmd.arg("--lavfi-complex=[aid1]asplit[ao][v];[v]showwaves=s=1280x720:mode=line:colors=cyan[vo]");
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
    }

    // Apply cookies to mpv (passed to ytdl-hook)
    match &settings.cookie_mode {
        crate::model::settings::CookieMode::Browser(b) => {
            cmd.arg(format!("--ytdl-raw-options=cookies-from-browser={}", b));
        }
        crate::model::settings::CookieMode::Netscape(p) => {
            cmd.arg(format!("--ytdl-raw-options=cookies={}", p.to_string_lossy()));
        }
        crate::model::settings::CookieMode::Json(p) => {
            let mut tmp = p.clone();
            tmp.set_extension("netscape.tmp");
            cmd.arg(format!("--ytdl-raw-options=cookies={}", tmp.to_string_lossy()));
        }
        crate::model::settings::CookieMode::Off => {}
    }

    cmd.arg(url);
    let child = cmd.spawn()?;
    Ok(child)
}

pub fn play_audio(url: &str, settings: &Settings) -> Result<Child> {
    let mut cmd = Command::new(settings.mpv_cmd());
    cmd.arg("--no-video");
    cmd.arg("--ytdl-format=bestaudio/best");
    cmd.kill_on_drop(true);

    // Common IPC setup
    let socket_path = if cfg!(windows) {
        format!(r"\\.\pipe\rataplay-mpv-{}", std::process::id())
    } else {
        format!("/tmp/rataplay-mpv-{}.sock", std::process::id())
    };
    cmd.arg(format!("--input-ipc-server={}", socket_path));
    cmd.arg("--idle=yes");

    // Apply cookies to mpv (passed to ytdl-hook)
    match &settings.cookie_mode {
        crate::model::settings::CookieMode::Browser(b) => {
            cmd.arg(format!("--ytdl-raw-options=cookies-from-browser={}", b));
        }
        crate::model::settings::CookieMode::Netscape(p) => {
            cmd.arg(format!("--ytdl-raw-options=cookies={}", p.to_string_lossy()));
        }
        crate::model::settings::CookieMode::Json(p) => {
            let mut tmp = p.clone();
            tmp.set_extension("netscape.tmp");
            cmd.arg(format!("--ytdl-raw-options=cookies={}", tmp.to_string_lossy()));
        }
        crate::model::settings::CookieMode::Off => {}
    }

    cmd.arg(url);

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn()?;
    Ok(child)
}
