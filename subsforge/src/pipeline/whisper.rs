use std::path::Path;
use tokio::process::Command;
use tracing::info;

use crate::config::WhisperConfig;
use crate::error::{SubsForgeError, Result};

pub struct WhisperResult {
    pub srt_content: String,
    pub detected_language: Option<String>,
}

/// Transcribe a WAV audio file using whisper-cli, returning SRT content.
pub async fn transcribe(config: &WhisperConfig, audio_path: &Path) -> Result<WhisperResult> {
    info!(audio = %audio_path.display(), "transcribing with whisper");

    // whisper-cli outputs SRT-style to stdout by default
    let mut cmd = Command::new(&config.binary);
    cmd.arg("-m").arg(&config.model)
        .arg("-f").arg(audio_path)
        .arg("--output-srt")
        .arg("--no-prints");

    if config.language != "auto" {
        cmd.arg("-l").arg(&config.language);
    }

    for arg in &config.extra_args {
        cmd.arg(arg);
    }

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(SubsForgeError::Whisper(stderr.to_string()));
    }

    // whisper-cli --output-srt writes to {input}.srt
    let srt_path = audio_path.with_extension("wav.srt");
    let srt_content = if srt_path.exists() {
        let content = tokio::fs::read_to_string(&srt_path).await?;
        tokio::fs::remove_file(&srt_path).await.ok();
        content
    } else {
        // Fallback: parse stdout (whisper outputs timestamps to stdout)
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout_to_srt(&stdout)
    };

    // Try to detect language from stderr logs
    let stderr = String::from_utf8_lossy(&output.stderr);
    let detected_language = parse_detected_language(&stderr);

    Ok(WhisperResult {
        srt_content,
        detected_language,
    })
}

/// Convert whisper stdout format [HH:MM:SS.mmm --> HH:MM:SS.mmm] text
/// to standard SRT format.
fn stdout_to_srt(stdout: &str) -> String {
    let mut entries = Vec::new();
    let mut seq = 1u32;

    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            if let Some(rest) = line.strip_prefix('[') {
                if let Some((timestamps, text)) = rest.split_once(']') {
                    let text = text.trim();
                    if text.is_empty() {
                        continue;
                    }
                    // Convert HH:MM:SS.mmm to HH:MM:SS,mmm
                    let ts = timestamps.replace('.', ",");
                    entries.push(format!("{seq}\n{ts}\n{text}"));
                    seq += 1;
                }
            }
        }
    }

    entries.join("\n\n") + "\n"
}

fn parse_detected_language(stderr: &str) -> Option<String> {
    // whisper-cli logs: "auto-detected language: en (p = 0.97)"
    for line in stderr.lines() {
        if let Some(rest) = line.strip_suffix(')') {
            if let Some(idx) = rest.rfind("auto-detected language: ") {
                let lang_part = &rest[idx + 24..];
                if let Some(lang) = lang_part.split_whitespace().next() {
                    return Some(lang.to_string());
                }
            }
        }
    }
    None
}
