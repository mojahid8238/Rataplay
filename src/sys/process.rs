use anyhow::Result;
use std::process::{Command, Stdio};

pub fn play_video(url: &str, in_terminal: bool) -> Result<()> {
    let mut cmd = Command::new("mpv");
    cmd.arg(url);

    if in_terminal {
        cmd.arg("--vo=tct");
        cmd.arg("--quiet");
    }

    if in_terminal {
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
    } else {
        // Detached / background
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
    }

    let mut child = cmd.spawn()?;
    child.wait()?;

    Ok(())
}

pub fn play_audio(url: &str) -> Result<()> {
    let mut cmd = Command::new("mpv");
    cmd.arg("--no-video");
    cmd.arg(url);

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = cmd.spawn()?;
    child.wait()?;

    Ok(())
}
