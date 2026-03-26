use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::api::AppState;
use crate::db;
use crate::models::{Job, JobWithTranslations, Stats};

#[derive(Deserialize)]
pub struct ListParams {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_jobs(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<Job>>, StatusCode> {
    let limit = params.limit.unwrap_or(50);
    db::get_jobs(&state.pool, params.status.as_deref(), limit)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<JobWithTranslations>, StatusCode> {
    let job = db::get_job(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let translations = db::get_translations_for_job(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(JobWithTranslations { job, translations }))
}

#[derive(Deserialize)]
pub struct CreateJobRequest {
    pub media_path: String,
    pub title: Option<String>,
}

pub async fn create_job(
    State(state): State<AppState>,
    Json(body): Json<CreateJobRequest>,
) -> Result<Json<Job>, StatusCode> {
    let title = body.title.as_deref().unwrap_or("Manual job");

    let job_id = db::create_job(
        &state.pool,
        &body.media_path,
        "manual",
        None,
        Some(title),
        &state.config.general.target_languages,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let job = db::get_job(&state.pool, job_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Spawn the processing pipeline in background
    let config = state.config.clone();
    let pool = state.pool.clone();
    let client = state.client.clone();
    let media_path = std::path::PathBuf::from(&body.media_path);

    tokio::spawn(async move {
        if let Err(e) = crate::pipeline::process_job(&config, &pool, &client, job_id, &media_path).await {
            tracing::error!(job_id, error = %e, "background job failed");
        }
    });

    Ok(Json(job))
}

pub async fn retry_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    db::retry_job(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Re-process in background
    let config = state.config.clone();
    let pool = state.pool.clone();
    let client = state.client.clone();

    tokio::spawn(async move {
        if let Some(job) = db::get_job(&pool, id).await.ok().flatten() {
            let media_path = std::path::PathBuf::from(&job.media_path);
            if let Err(e) = crate::pipeline::process_job(&config, &pool, &client, id, &media_path).await {
                tracing::error!(job_id = id, error = %e, "retry job failed");
            }
        }
    });

    Ok(StatusCode::ACCEPTED)
}

pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<Stats>, StatusCode> {
    db::get_stats(&state.pool)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
