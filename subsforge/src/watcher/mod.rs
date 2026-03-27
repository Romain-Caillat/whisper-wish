pub mod radarr;
pub mod sonarr;

use std::sync::Arc;
use reqwest::Client;
use sqlx::{Pool, Sqlite};
use tokio::time;
use tracing::{info, error};

use crate::config::Config;
use crate::db;
use crate::pipeline;

/// Start the polling loop that watches Sonarr/Radarr for new media.
pub async fn start(config: Arc<Config>, pool: Pool<Sqlite>, client: Client) {
    let interval = time::Duration::from_secs(config.general.poll_interval_minutes * 60);
    info!(interval_minutes = config.general.poll_interval_minutes, "watcher started");

    // Run immediately on start, then on interval
    loop {
        if let Err(e) = poll_and_process(&config, &pool, &client).await {
            error!(error = %e, "polling cycle failed");
        }
        time::sleep(interval).await;
    }
}

async fn poll_and_process(
    config: &Config,
    pool: &Pool<Sqlite>,
    client: &Client,
) -> anyhow::Result<()> {
    let mut media_items = Vec::new();

    // Fetch full library from Sonarr
    if let Some(sonarr_config) = &config.sonarr {
        match sonarr::fetch_all(sonarr_config, client).await {
            Ok(items) => {
                for item in items {
                    media_items.push(("sonarr", item));
                }
            }
            Err(e) => error!(error = %e, "failed to fetch from Sonarr"),
        }
    }

    // Fetch full library from Radarr
    if let Some(radarr_config) = &config.radarr {
        match radarr::fetch_all(radarr_config, client).await {
            Ok(items) => {
                for item in items {
                    media_items.push(("radarr", item));
                }
            }
            Err(e) => error!(error = %e, "failed to fetch from Radarr"),
        }
    }

    // Process each new item
    for (source, item) in media_items {
        let job_id = db::create_job(
            pool,
            &item.path,
            source,
            Some(item.source_id),
            Some(&item.title),
            &config.general.target_languages,
        ).await?;

        // Check if already processed
        if let Some(job) = db::get_job(pool, job_id).await? {
            if job.status != "pending" {
                continue;
            }
        }

        // Map remote path to local
        let local_path = match config.map_path(&item.path) {
            Ok(p) => p,
            Err(e) => {
                error!(path = item.path, error = %e, "path mapping failed");
                db::update_job_status(pool, job_id, "failed", Some(&e.to_string())).await?;
                continue;
            }
        };

        if !local_path.exists() {
            let msg = format!("file not found: {}", local_path.display());
            error!(msg);
            db::update_job_status(pool, job_id, "failed", Some(&msg)).await?;
            continue;
        }

        info!(title = item.title, path = %local_path.display(), "processing new media");

        if let Err(e) = pipeline::process_job(config, pool, client, job_id, &local_path).await {
            error!(title = item.title, error = %e, "pipeline failed");
        }
    }

    Ok(())
}
