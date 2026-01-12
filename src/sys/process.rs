use std::process::{Command, Stdio};
use anyhow::Result;

pub fn play_video(url: &str, in_terminal: bool) -> Result<()> {
    let mut cmd = Command::new("mpv");
    cmd.arg(url);
    
    if in_terminal {
        cmd.arg("--vo=tct");
    }

    cmd.stdin(Stdio::inherit())
       .stdout(Stdio::inherit())
       .stderr(Stdio::inherit());

    let mut child = cmd.spawn()?;
    child.wait()?;

    Ok(())
}

pub fn play_audio(url: &str) -> Result<()> {
    let mut cmd = Command::new("mpv");
    cmd.arg("--no-video");
    cmd.arg(url);
    
    cmd.stdin(Stdio::inherit())
       .stdout(Stdio::inherit())
       .stderr(Stdio::inherit());

    let mut child = cmd.spawn()?;
    child.wait()?;

    Ok(())
}

pub fn download_video(url: &str, format_id: &str) -> Result<()> {
    // yt-dlp -f <format_id> <url>
    // We should probably run this in a terminal or show output.
    // For now, let's behave like play: take over terminal to show output.
    let mut cmd = Command::new("yt-dlp");
    cmd.arg("-f").arg(format_id);
    cmd.arg("-o").arg("%(title)s.%(ext)s"); // Download to current dir
    cmd.arg(url);
    
    cmd.stdin(Stdio::inherit())
       .stdout(Stdio::inherit())
       .stderr(Stdio::inherit());

    let mut child = cmd.spawn()?;
    child.wait()?;

    Ok(())
}
