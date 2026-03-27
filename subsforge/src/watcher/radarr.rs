use reqwest::Client;
use serde::Deserialize;
use tracing::info;

use crate::config::ArrConfig;
use super::sonarr::MediaItem;

#[derive(Deserialize)]
struct Movie {
    id: i64,
    title: Option<String>,
    year: Option<i64>,
    #[serde(rename = "hasFile")]
    has_file: Option<bool>,
    #[serde(rename = "movieFile")]
    movie_file: Option<MovieFile>,
}

#[derive(Deserialize)]
struct MovieFile {
    path: Option<String>,
}

/// Fetch all movies with files from Radarr.
pub async fn fetch_all(config: &ArrConfig, client: &Client) -> anyhow::Result<Vec<MediaItem>> {
    let movies: Vec<Movie> = client
        .get(format!("{}/api/v3/movie", config.url))
        .header("X-Api-Key", &config.api_key)
        .send()
        .await?
        .json()
        .await?;

    let mut items = Vec::new();

    for movie in &movies {
        if movie.has_file != Some(true) {
            continue;
        }

        let path = movie.movie_file.as_ref().and_then(|f| f.path.as_deref());

        if let Some(path) = path {
            let title = match (movie.title.as_deref(), movie.year) {
                (Some(t), Some(y)) => format!("{t} ({y})"),
                (Some(t), None) => t.to_string(),
                _ => "Unknown".to_string(),
            };

            items.push(MediaItem {
                path: path.to_string(),
                title,
                source_id: movie.id,
            });
        }
    }

    info!(count = items.len(), "fetched all movies from Radarr");
    Ok(items)
}
