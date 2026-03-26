pub mod health;
pub mod jobs;

use std::sync::Arc;
use axum::{Router, routing::get, routing::post};
use reqwest::Client;
use sqlx::{Pool, Sqlite};
use tower_http::cors::CorsLayer;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: Pool<Sqlite>,
    pub client: Client,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health::health))
        .route("/api/jobs", get(jobs::list_jobs).post(jobs::create_job))
        .route("/api/jobs/{id}", get(jobs::get_job))
        .route("/api/jobs/{id}/retry", post(jobs::retry_job))
        .route("/api/stats", get(jobs::get_stats))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
