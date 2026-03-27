use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

use crate::config::ArrConfig;

#[derive(Debug)]
pub struct MediaItem {
    pub path: String,
    pub title: String,
    pub source_id: i64,
}

#[derive(Deserialize)]
struct Series {
    id: i64,
    title: Option<String>,
}

#[derive(Deserialize)]
struct EpisodeFile {
    id: i64,
    path: Option<String>,
    #[serde(rename = "seasonNumber")]
    season_number: Option<i64>,
}

/// Fetch all episodes with files from Sonarr by scanning the full library.
pub async fn fetch_all(config: &ArrConfig, client: &Client) -> anyhow::Result<Vec<MediaItem>> {
    // 1. Get all series
    let series_list: Vec<Series> = client
        .get(format!("{}/api/v3/series", config.url))
        .header("X-Api-Key", &config.api_key)
        .send()
        .await?
        .json()
        .await?;

    info!(count = series_list.len(), "found series in Sonarr");

    let mut items = Vec::new();

    // 2. For each series, get all episode files
    for series in &series_list {
        let series_title = series.title.as_deref().unwrap_or("Unknown");

        let files: Vec<EpisodeFile> = match client
            .get(format!("{}/api/v3/episodefile", config.url))
            .header("X-Api-Key", &config.api_key)
            .query(&[("seriesId", &series.id.to_string())])
            .send()
            .await
        {
            Ok(resp) => match resp.json().await {
                Ok(f) => f,
                Err(e) => {
                    warn!(series = series_title, error = %e, "failed to parse episode files");
                    continue;
                }
            },
            Err(e) => {
                warn!(series = series_title, error = %e, "failed to fetch episode files");
                continue;
            }
        };

        for file in &files {
            if let Some(path) = &file.path {
                let season = file.season_number.unwrap_or(0);
                // Extract episode info from filename
                let filename = std::path::Path::new(path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let title = format!("{series_title} S{season:02} - {filename}");

                items.push(MediaItem {
                    path: path.clone(),
                    title,
                    source_id: file.id,
                });
            }
        }
    }

    info!(count = items.len(), "fetched all episodes from Sonarr");
    Ok(items)
}
