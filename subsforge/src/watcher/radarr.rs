use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

use crate::config::ArrConfig;
use super::sonarr::MediaItem;

#[derive(Deserialize)]
struct HistoryRecord {
    #[serde(rename = "movieId")]
    movie_id: Option<i64>,
    #[serde(rename = "sourceTitle")]
    source_title: Option<String>,
}

#[derive(Deserialize)]
struct HistoryResponse {
    records: Vec<HistoryRecord>,
}

#[derive(Deserialize)]
struct Movie {
    title: Option<String>,
    year: Option<i64>,
    #[serde(rename = "movieFile")]
    movie_file: Option<MovieFile>,
}

#[derive(Deserialize)]
struct MovieFile {
    path: Option<String>,
}

/// Fetch recently imported movies from Radarr.
pub async fn fetch_recent(config: &ArrConfig, client: &Client) -> anyhow::Result<Vec<MediaItem>> {
    let history: HistoryResponse = client
        .get(format!("{}/api/v3/history", config.url))
        .header("X-Api-Key", &config.api_key)
        .query(&[
            ("eventType", "downloadFolderImported"),
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
        let movie_id = match record.movie_id {
            Some(id) if id > 0 => id,
            _ => continue,
        };

        let movie: Movie = match client
            .get(format!("{}/api/v3/movie/{}", config.url, movie_id))
            .header("X-Api-Key", &config.api_key)
            .send()
            .await
        {
            Ok(resp) => match resp.json().await {
                Ok(m) => m,
                Err(e) => {
                    warn!(movie_id, error = %e, "failed to parse movie");
                    continue;
                }
            },
            Err(e) => {
                warn!(movie_id, error = %e, "failed to fetch movie");
                continue;
            }
        };

        let path = movie.movie_file
            .and_then(|f| f.path);

        if let Some(path) = path {
            let title = match (movie.title.as_deref(), movie.year) {
                (Some(t), Some(y)) => format!("{t} ({y})"),
                (Some(t), None) => t.to_string(),
                _ => "Unknown".to_string(),
            };

            items.push(MediaItem {
                path,
                title,
                source_id: movie_id,
            });
        }
    }

    info!(count = items.len(), "fetched recent movies from Radarr");
    Ok(items)
}
