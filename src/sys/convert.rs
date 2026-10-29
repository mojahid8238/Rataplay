use crate::model::convert::ConvertTarget;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Build a non-clobbering output path next to the input file.
pub fn output_path(input: &Path, ext: &str) -> PathBuf {
    let input_ext = input.extension().and_then(|e| e.to_str()).unwrap_or("");
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    // Never overwrite the source, even when the target format is the same.
    let base_stem = if input_ext.eq_ignore_ascii_case(ext) {
        format!("{}.converted", stem)
    } else {
        stem.to_string()
    };

    let mut candidate = input.with_file_name(format!("{}.{}", base_stem, ext));
    let mut counter = 1;
    while candidate.exists() {
        candidate = input.with_file_name(format!("{}.{}.{}", base_stem, counter, ext));
        counter += 1;
    }
    candidate
}

pub async fn ffmpeg_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("-version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Convert a local file with ffmpeg and return the path of the new file.
pub async fn convert_file(
    input: &Path,
    target: ConvertTarget,
    ffmpeg_cmd: &str,
) -> Result<PathBuf> {
    if !input.exists() {
        bail!("Input file not found: {}", input.display());
    }

    let output = output_path(input, target.ext());

    log::info!(
        "Converting local file {:?} to {} ({:?})",
        input,
        target.label(),
        output
    );

    let result = Command::new(ffmpeg_cmd)
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-y")
        .arg("-i")
        .arg(input)
        .args(target.ffmpeg_args())
        .arg(&output)
        .output()
        .await
        .context(format!(
            "Failed to run ffmpeg at '{}'. Is it installed and in your PATH?",
            ffmpeg_cmd
        ))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let reason = stderr
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("unknown error");
        let _ = tokio::fs::remove_file(&output).await;
        bail!("ffmpeg failed: {}", reason);
    }

    Ok(output)
}
