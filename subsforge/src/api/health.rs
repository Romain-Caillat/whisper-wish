use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::api::AppState;
use crate::pipeline::translator;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub whisper: bool,
    pub translator: bool,
    pub database: bool,
}

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_ok = sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .is_ok();

    let translator_ok = translator::health_check(&state.config.translator, &state.client).await;

    let whisper_ok = state.config.whisper.binary.exists()
        && state.config.whisper.model.exists();

    let status = if db_ok && translator_ok && whisper_ok {
        "healthy"
    } else {
        "degraded"
    };

    Json(HealthResponse {
        status,
        whisper: whisper_ok,
        translator: translator_ok,
        database: db_ok,
    })
}
