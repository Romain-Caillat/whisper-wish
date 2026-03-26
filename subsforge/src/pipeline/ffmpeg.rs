use std::path::Path;
use tokio::process::Command;
use tracing::info;

use crate::config::FfmpegConfig;
use crate::error::{SubsForgeError, Result};

/// Extract audio from a media file to a WAV file (16kHz, mono, PCM s16le).
pub async fn extract_audio(
    config: &FfmpegConfig,
    input: &Path,
    output: &Path,
) -> Result<()> {
    info!(input = %input.display(), output = %output.display(), "extracting audio");

    let mut cmd = Command::new(&config.binary);
    cmd.arg("-i").arg(input)
        .arg("-vn")
        .arg("-ar").arg("16000")
        .arg("-ac").arg("1")
        .arg("-c:a").arg("pcm_s16le");

    if config.audio_track > 0 {
        cmd.arg("-map").arg(format!("0:a:{}", config.audio_track));
    }

    cmd.arg("-y")
        .arg("-loglevel").arg("error")
        .arg(output);

    let output_result = cmd.output().await?;

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        return Err(SubsForgeError::Ffmpeg(stderr.to_string()));
    }

    Ok(())
}
