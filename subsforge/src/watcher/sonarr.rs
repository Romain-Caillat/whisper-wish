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
struct HistoryRecord {
    #[serde(rename = "sourceTitle")]
    source_title: Option<String>,
    #[serde(rename = "episodeId")]
    episode_id: Option<i64>,
    #[serde(rename = "eventType")]
    event_type: Option<String>,
}

#[derive(Deserialize)]
struct HistoryResponse {
    records: Vec<HistoryRecord>,
}

#[derive(Deserialize)]
struct EpisodeFile {
    path: Option<String>,
}

#[derive(Deserialize)]
struct Episode {
    #[serde(rename = "episodeFileId")]
    episode_file_id: Option<i64>,
    title: Option<String>,
    #[serde(rename = "seasonNumber")]
    season_number: Option<i64>,
    #[serde(rename = "episodeNumber")]
    episode_number: Option<i64>,
    series: Option<Series>,
}

#[derive(Deserialize)]
struct Series {
    title: Option<String>,
}

/// Fetch recently imported episodes from Sonarr.
pub async fn fetch_recent(config: &ArrConfig, client: &Client) -> anyhow::Result<Vec<MediaItem>> {
    let since = chrono::Utc::now() - chrono::Duration::days(config.lookback_days as i64);
    let _since_str = since.format("%Y-%m-%d").to_string();

    let history: HistoryResponse = client
        .get(format!("{}/api/v3/history", config.url))
        .header("X-Api-Key", &config.api_key)
        .query(&[
            ("eventType", "grabbed"),
            ("sortKey", "date"),
            ("sortDirection", "descending"),
            ("pageSize", "50"),
        ])
        .send()
        .await?
        .json()
        .await?;

    let mut items = Vec::new();

    for record in &history.records {
        let episode_id = match record.episode_id {
            Some(id) if id > 0 => id,
            _ => continue,
        };

        // Fetch episode details to get file path
        let episode: Episode = match client
            .get(format!("{}/api/v3/episode/{}", config.url, episode_id))
            .header("X-Api-Key", &config.api_key)
            .send()
            .await
        {
            Ok(resp) => match resp.json().await {
                Ok(ep) => ep,
                Err(e) => {
                    warn!(episode_id, error = %e, "failed to parse episode");
                    continue;
                }
            },
            Err(e) => {
                warn!(episode_id, error = %e, "failed to fetch episode");
                continue;
            }
        };

        let file_id = match episode.episode_file_id {
            Some(id) if id > 0 => id,
            _ => continue,
        };

        let episode_file: EpisodeFile = match client
            .get(format!("{}/api/v3/episodefile/{}", config.url, file_id))
            .header("X-Api-Key", &config.api_key)
            .send()
            .await
        {
            Ok(resp) => match resp.json().await {
                Ok(ef) => ef,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        if let Some(path) = episode_file.path {
            let series_title = episode.series.as_ref()
                .and_then(|s| s.title.as_deref())
                .unwrap_or("Unknown");
            let season = episode.season_number.unwrap_or(0);
            let ep_num = episode.episode_number.unwrap_or(0);
            let ep_title = episode.title.as_deref().unwrap_or("");

            let title = format!("{series_title} S{season:02}E{ep_num:02} - {ep_title}");

            items.push(MediaItem {
                path,
                title,
                source_id: episode_id,
            });
        }
    }

    info!(count = items.len(), "fetched recent episodes from Sonarr");
    Ok(items)
}
