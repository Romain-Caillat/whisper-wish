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

    let mut cmd = Command::new(&config.binary);
    cmd.arg("-m").arg(&config.model)
        .arg("-f").arg(audio_path)
        .arg("--output-srt");

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
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout_to_srt(&stdout)
    };

    // Detect language from stderr (whisper prints "lang = xx" in processing line)
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
    for line in stderr.lines() {
        // Format: "auto-detected language: en (p = 0.97)"
        if let Some(idx) = line.find("auto-detected language: ") {
            let lang_part = &line[idx + 24..];
            if let Some(lang) = lang_part.split_whitespace().next() {
                return Some(lang.to_string());
            }
        }
        // Format: "lang = en, task = transcribe"
        if let Some(idx) = line.find("lang = ") {
            let lang_part = &line[idx + 7..];
            if let Some(lang) = lang_part.split(',').next() {
                let lang = lang.trim();
                if !lang.is_empty() {
                    return Some(lang.to_string());
                }
            }
        }
    }
    None
}
