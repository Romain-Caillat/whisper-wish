use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    pub server: ServerConfig,
    pub whisper: WhisperConfig,
    pub ffmpeg: FfmpegConfig,
    pub translator: TranslatorConfig,
    pub sonarr: Option<ArrConfig>,
    pub radarr: Option<ArrConfig>,
    pub path_mappings: Vec<PathMapping>,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeneralConfig {
    pub poll_interval_minutes: u64,
    pub target_languages: Vec<String>,
    #[serde(default = "default_true")]
    pub save_original_srt: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WhisperConfig {
    pub binary: PathBuf,
    pub model: PathBuf,
    #[serde(default = "default_auto")]
    pub language: String,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FfmpegConfig {
    pub binary: PathBuf,
    #[serde(default)]
    pub audio_track: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TranslatorConfig {
    pub endpoint: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ArrConfig {
    pub url: String,
    pub api_key: String,
    #[serde(default = "default_lookback")]
    pub lookback_days: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PathMapping {
    pub remote_prefix: String,
    pub local_prefix: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&content)?;
        // Expand ~ in paths
        config.whisper.model = expand_tilde(&config.whisper.model);
        config.whisper.binary = expand_tilde(&config.whisper.binary);
        config.ffmpeg.binary = expand_tilde(&config.ffmpeg.binary);
        config.database.url = config.database.url.replace(
            '~',
            &dirs_home().to_string_lossy(),
        );
        Ok(config)
    }

    pub fn map_path(&self, remote_path: &str) -> crate::error::Result<PathBuf> {
        for mapping in &self.path_mappings {
            if remote_path.starts_with(&mapping.remote_prefix) {
                let relative = &remote_path[mapping.remote_prefix.len()..];
                return Ok(PathBuf::from(&mapping.local_prefix).join(relative.trim_start_matches('/')));
            }
        }
        Err(crate::error::SubsForgeError::PathMapping(remote_path.to_string()))
    }
}

fn expand_tilde(path: &Path) -> PathBuf {
    if let Ok(stripped) = path.strip_prefix("~") {
        dirs_home().join(stripped)
    } else {
        path.to_path_buf()
    }
}

fn dirs_home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()))
}

fn default_true() -> bool { true }
fn default_host() -> String { "0.0.0.0".to_string() }
fn default_port() -> u16 { 8385 }
fn default_auto() -> String { "auto".to_string() }
fn default_timeout() -> u64 { 300 }
fn default_retries() -> u32 { 3 }
fn default_lookback() -> u32 { 7 }
