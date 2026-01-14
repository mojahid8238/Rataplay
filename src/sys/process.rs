use anyhow::Result;
use std::process::Stdio;
use tokio::process::{Child, Command};

pub fn play_video(url: &str, in_terminal: bool, user_agent: Option<&str>) -> Result<Child> {
    let mut cmd = Command::new("mpv");
    cmd.arg(url);
    cmd.kill_on_drop(true);

    if let Some(ua) = user_agent {
        cmd.arg(format!("--user-agent={}", ua));
        // Also set ytdl=no to avoid double extraction which often causes 403
        cmd.arg("--ytdl=no");
    }

    if in_terminal {
        cmd.arg("--vo=tct");
        cmd.arg("--really-quiet");
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
        let socket_path = format!("/tmp/rataplay-mpv-{}.sock", std::process::id());
        cmd.arg(format!("--input-ipc-server={}", socket_path));
        cmd.arg("--idle=yes");
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
    }

    let child = cmd.spawn()?;
    Ok(child)
}

pub fn play_audio(url: &str) -> Result<Child> {
    let mut cmd = Command::new("mpv");
    cmd.arg("--no-video");
    cmd.arg("--ytdl-format=bestaudio/best");
    cmd.kill_on_drop(true);

    let socket_path = format!("/tmp/rataplay-mpv-{}.sock", std::process::id());
    cmd.arg(format!("--input-ipc-server={}", socket_path));
    cmd.arg("--idle=yes");
    cmd.arg(url);

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn()?;
    Ok(child)
}
